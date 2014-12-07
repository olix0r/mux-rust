//! An implementation of mux in Rust.
//! See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala

#![crate_name = "mux"]
#![experimental]
#![crate_type = "rlib"]
#![crate_type = "dylib"]

pub mod misc;
pub mod proto;
pub mod reader;
pub mod writer;
