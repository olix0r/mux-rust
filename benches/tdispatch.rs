extern crate test;
extern crate mux;

use mux::misc::{Context, Dentry, Dtab};
use mux::proto::{types, Message, Tdispatch, Tag};
use mux::reader::MessageReader;
use mux::writer::MessageWriter;
use std::io::{BufReader, MemWriter};
use test::Bencher;


static TDISPATCH_BUF: &'static [u8] = [
    types::TDISPATCH as u8,
    0, 1, 2, // tag

    // contexts:
    0, 2, // 2 contexts

    // context 0 key
    0, 4, // length
    1, 2, 3, 4,

    // context 0 val
    0, 2, // length
    6, 7,

    // context 1 key
    0, 2, // length
    3, 4,

    // context 1 val
    0, 3, // length
    6, 7, 8,

    // dst
    0, 4, // length
    '/' as u8, 66, 65, 68, // "/BAD"

    // dtab: /BAD => /DAD
    0, 1, // length
    0, 4, // source length
    '/' as u8, 66, 65, 68, // "/BAD"
    0, 4, // tree length
    '/' as u8, 68, 65, 68, // "/DAD"

    // data: [0 .. 20)
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19];


#[inline]
fn read_tdispatch() {
    let mut reader = BufReader::new(TDISPATCH_BUF);
    reader.read_message().ok();
}

#[inline]
fn write_tdispatch(msg: &Message) {
    let mut writer = MemWriter::new();
    writer.write_message(msg).ok();
}

#[bench]
fn bench_read_tdispatch(bench: &mut Bencher) {
    bench.iter(|| read_tdispatch());
    bench.bytes = TDISPATCH_BUF.len() as u64;
}

#[bench]
fn bench_write_tdispatch(bench: &mut Bencher) {
    let msg = Tdispatch(
        Tag(0, 1, 2),
        vec![Context::new(vec![1,2,3,4], vec![6,7]),
             Context::new(vec![3,4], vec![6,7,8])],
        "/BAD".to_string(),
        Dtab(vec![Dentry::new("/BAD".to_string(), "/DAD".to_string())]),
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    bench.iter(|| write_tdispatch(&msg));
    bench.bytes = TDISPATCH_BUF.len() as u64;
}
