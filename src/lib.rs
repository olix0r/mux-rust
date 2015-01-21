//! An implementation of mux in Rust.
//! See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala

#![crate_name = "mux"]
#![experimental]
#![crate_type = "rlib"]
#![crate_type = "dylib"]

pub use proto::{Tag, Msg, Tmsg, Rmsg};
pub use reader::MuxReader;
pub use writer::MuxWriter;

pub mod misc;

mod proto;
mod reader;
mod writer;
