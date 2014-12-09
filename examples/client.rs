//! Mux echo client
#![feature(globs)]

extern crate mux;

use std::io::net::tcp::TcpStream;
use std::io::timer::Timer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUint, SeqCst};
use std::time::Duration;

use mux::misc::Dtab;
use mux::proto::*;
use mux::reader::MessageReader;
use mux::writer::MessageWriter;

fn main() {
    let counter_arc = Arc::new(AtomicUint::new(0));
    let ctr = counter_arc.clone();
    // let bytes_arc = Arc::new(AtomicUint::new(0));
    // let bytes_ctr = bytes_arc.clone();
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

    for id in range(0u, 1) {
        let ctr = counter_arc.clone();
        //let bytes_ctr = bytes_arc.clone();

        let tmsg = Tdispatch(Vec::new(), String::new(), Dtab(Vec::new()),
                             b"nope".to_vec());

        spawn(proc() {
            loop {
                match TcpStream::connect("127.0.0.1", 6666) {
                    Err(_) => (),
                    Ok(mut conn) => {
                        println!("-- {}: connected: {}", id, conn.peer_name());
                        //conn.set_read_timeout(Some(50));
                        //conn.set_write_timeout(Some(50));

                        loop {
                            //println!("{}: writing: {}", id, tmsg)
                            match conn.write_mux_frame(Tag(1,2,3), &tmsg) {
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
                            //println!("{}: wrote: {}", id, tmsg);

                            let Framed(_, _) = match conn.read_mux_frame() {
                                Err(ioe) => {
                                    println!("{}: read error: {}", id, ioe);
                                    break;
                                },

                                Ok(framed) => framed
                            };
                            //println!("{}: read: {}", id, msg);
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
}

