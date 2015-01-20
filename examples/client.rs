//! Stupid awful single-threaded Mux client

extern crate mux;

use std::clone::Clone;
use std::io::net::tcp::TcpStream;
use std::io::timer::Timer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUint, Ordering};
use std::thread::Thread;
use std::time::Duration;

use mux::*;
use mux::misc::*;

#[allow(unstable)]
fn main() {
    let dst = "127.0.0.1:6666";
    let ctr = Arc::new(AtomicUint::new(0));
    let read_ctr = ctr.clone();
    Thread::spawn(move|| {
        let mut timer = Timer::new().unwrap();
        let mut last: usize = 0;
        loop {
            let current = read_ctr.load(Ordering::SeqCst);
            println!("{} rps", (current - last) / 2);
            last = current;
            timer.sleep(Duration::seconds(2));
        }
    });

    let tmsg = Tmsg::Dispatch(
        Vec::new(),
        "/path".to_string(),
        Dtab(vec![Dentry::new("/from".to_string(), "/to".to_string())]),
        b"nope".to_vec());

    loop {
        match TcpStream::connect(dst) {
            Err(_) => println!("connect error"),

            Ok(mut conn) => {
                let id = format!("{}", conn.socket_name().unwrap());
                println!("-- {}: connected: {}", dst, id);
                //conn.set_read_timeout(Some(50));
                //conn.set_write_timeout(Some(50));

                loop {
                    //println!("{}: writing: {}", id, tmsg)
                    match conn.write_mux_framed_tmsg(&Tag(1,2,3), &tmsg) {
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

                    let (_, _) = match conn.read_mux_framed_rmsg() {
                        Err(ioe) => {
                            println!("{}: read error: {}", id, ioe);
                            break;
                        },

                        Ok(framed) => framed
                    };
                    //println!("{}: read: {}", id, msg);

                    ctr.fetch_add(1, Ordering::SeqCst);
                }

                conn.close_read().ok();
                conn.close_write().ok();
                println!("-- {}: disconnected", id);
            }
        }
    }
}

