//! Simplistic mux echo server
// Only serves one client at a time.  Kinda sucks.  A lot.

extern crate mux;

use std::io::{Acceptor, Listener};
use std::io::net::tcp::TcpListener;
use std::io::timer::Timer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUint, Ordering};
use std::thread::Thread;
use std::time::Duration;

use mux::*;

#[allow(unstable)]
fn main() {
    let ctr_arc = Arc::new(AtomicUint::new(0));

    // log rps periodically:
    let read_ctr = ctr_arc.clone();
    Thread::spawn(move|| {
        let mut timer = Timer::new().unwrap();
        let mut last: usize = 0;
        loop {
            let current = read_ctr.load(Ordering::SeqCst);
            let delta = (current - last) / 60;
            if delta > 0 {
                println!("{} rps", delta);
                last = current;
            }
            timer.sleep(Duration::seconds(60));
        }
    });

    let addr = "0.0.0.0:6666";
    let listener = TcpListener::bind(addr).unwrap();
    let mut acceptor = listener.listen();
    println!("serving on {}", addr);

    for conn in acceptor.incoming() {
        match conn {
            Err(_) => (),
            Ok(mut conn) => {
                let ctr = ctr_arc.clone();
                Thread::spawn(move|| {
                    let id = format!("{}", conn.peer_name().unwrap());
                    println!("-- {}: connected", id);
                    //conn.set_read_timeout(Some(50));
                    //conn.set_write_timeout(Some(50));

                    loop {
                        let (tag, req) = match conn.read_mux_framed_tmsg() {
                            Err(ioe) => {
                                println!("{}: read error: {}", id, ioe);
                                break;
                            },
                            Ok(framed) => framed,
                        };

                        let rsp = match req {
                            Tmsg::Req(_, body) => Rmsg::ReqOk(body),
                            Tmsg::Dispatch(ctxs, _, _, body) => Rmsg::DispatchOk(ctxs, body),
                            Tmsg::Drain => Rmsg::Drain,
                            Tmsg::Ping => Rmsg::Ping,
                            _ => Rmsg::Err("idk man".to_string()),
                        };

                        match conn.write_mux_framed_rmsg(&tag, &rsp) {
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

                        ctr.fetch_add(1, Ordering::SeqCst);
                    }

                    conn.close_read().ok();
                    conn.close_write().ok();
                    println!("-- {}: disconnected", id);
                });
            }
        }
    }
}
