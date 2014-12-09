//! Mux echo client
#![feature(globs)]

extern crate mux;

use std::io::net::tcp::TcpStream;
use std::io::timer::Timer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUint, SeqCst};
use std::time::Duration;

use mux::misc::*;
use mux::proto::*;
use mux::reader::MuxReader;
use mux::writer::MuxWriter;


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

    for id in range(0u, 3) {
        let ctr = counter_arc.clone();
        //let bytes_ctr = bytes_arc.clone();

        let tmsg = Tdispatch(
            Vec::new(),
            "/path".to_string(),
            Dtab(vec![
                Dentry::new("/from".to_string(), "/to".to_string())]),
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
                            match conn.write_mux_frame(Tag(1,2,3), tmsg.clone()) {
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

                            let (_, _) = match conn.read_mux_frame() {
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

