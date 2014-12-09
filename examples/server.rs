//! Mux echo servre
#![feature(globs)]

extern crate mux;

use std::io::{Acceptor, Listener, Reader};
use std::io::net::tcp::{TcpListener, TcpStream};
use std::io::timer::Timer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUint, SeqCst};
use std::sync::deque::{BufferPool, Empty, Abort, Data};
use std::time::Duration;

use mux::proto::*;
use mux::reader::MuxReader;
use mux::writer::MuxWriter;

fn main() {
    let mut listener = TcpListener::bind("0.0.0.0", 6666).unwrap();
    let addr = listener.socket_name().unwrap();

    let mut acceptor = listener.listen();
    println!("serving on {}", addr);

    let pool = BufferPool::<TcpStream>::new();
    let (worker, stealer) = pool.deque();

    let counter_arc = Arc::new(AtomicUint::new(0));
    let ctr = counter_arc.clone();
    spawn(proc() {
        let mut timer = Timer::new().unwrap();
        let mut last: uint = 0;
        loop {
            let current = ctr.load(SeqCst);
            println!("{} rps", (current - last) / 2);
            last = current;
            timer.sleep(Duration::seconds(2));
        }
    });

    for id in range(0u, 3) {
        let ctr = counter_arc.clone();
        let rx = stealer.clone();
        spawn(proc() {
            loop {
                match rx.steal() {
                    Empty | Abort => (),

                    Data(mut conn) => {
                        println!("-- {}: connected: {}", id, conn.peer_name());
                        //conn.set_read_timeout(Some(50));
                        //conn.set_write_timeout(Some(50));

                        loop {
                            //println!("-- {}: reading", id);
                            let (tag, req) = match conn.read_mux_frame() {
                                Err(ioe) => {
                                    println!("{}: read error: {}", id, ioe);
                                    break;
                                },

                                Ok(framed) => framed,
                            };
                            //println!("{}: read: {}", id, req);

                            let rsp = match req {
                                Treq(_, body) => RreqOk(body),
                                Tdispatch(ctxs, _, _, body) => RdispatchOk(ctxs, body),
                                Tdrain => Rdrain,
                                Tping => Rping,
                                _ => Rerr("idk man".to_string()),
                            };

                            //println!("{}: writing: {}", id, rsp);
                            match conn.write_mux_frame(tag, rsp) {
                                Err(ioe) => {
                                    println!("{}: write error: {}", id, ioe);
                                    break;
                                },
                                Ok(_) => ()
                            };
                            match conn.flush() {
                                Err(ioe) => {
                                    println!("{}: flush error: {}", id, ioe);
                                    break;
                                },
                                Ok(_) => ()
                            };
                            //println!("{}: wrote: {}", id, rsp)

                            ctr.fetch_add(1, SeqCst);
                        }

                        conn.close_read().ok();
                        conn.close_write().ok();
                        println!("-- {}: disconnected", id);
                    }
                }
            }
        });
    }

    for conn in acceptor.incoming() {
        match conn {
            Err(_) => (),

            Ok(stream) => worker.push(stream)
        }
    }
}
