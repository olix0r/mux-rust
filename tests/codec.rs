#![feature(globs)]
extern crate mux;

use std::io::{Reader, Writer, BufReader, MemWriter};

use mux::misc::{Context, Dentry, Dtab};
use mux::proto::{types, Message, Tdispatch, Tag};
use mux::reader::*;
use mux::writer::*;


static TDISPATCH_BUF: &'static [u8] = [
    0, 0, 0, 65, // frame size

    types::TDISPATCH as u8,
    4, 7, 9, // tag

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

static TAG: Tag = Tag(4, 7, 9);

fn read_frame<R: Reader>(reader: &mut R) -> (Tag, Message) {
    let bytes = reader.read_be_u32_frame().unwrap();
    let mut buf = BufReader::new(bytes.as_slice());
    buf.read_mux().unwrap()
}

fn write_frame<W: Writer>(writer: &mut W, msg: Message) {
    let mut buf = MemWriter::new();
    buf.write_mux(TAG, msg.clone()).ok();
    let vec = buf.unwrap();
    writer.write_be_u32_frame(vec.as_slice()).unwrap();
}

#[test]
fn codec_tdispatch() {
    let msg = Tdispatch(
        vec![Context::new(vec![1,2,3,4], vec![6,7]),
             Context::new(vec![3,4], vec![6,7,8])],
        "/BAD".to_string(),
        Dtab(vec![Dentry::new("/BAD".to_string(), "/DAD".to_string())]),
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);

    /* reader */ {
        let mut concat = Vec::with_capacity(TDISPATCH_BUF.len() * 2);
        concat.extend(TDISPATCH_BUF.iter().map(|&b| -> u8 { b }));
        concat.extend(TDISPATCH_BUF.iter().map(|&b| -> u8 { b }));

        let mut reader = BufReader::new(concat.as_slice());
        assert_eq!(read_frame(&mut reader), (TAG, msg.clone()));
        assert_eq!(read_frame(&mut reader), (TAG, msg.clone()));
    }

    /* writer */ {
        let mut writer = MemWriter::new();
        write_frame(&mut writer, msg.clone());
        write_frame(&mut writer, msg.clone());
        write_frame(&mut writer, msg.clone());
        let buf = writer.unwrap();

        let mut reader = BufReader::new(buf.as_slice());
        assert_eq!(read_frame(&mut reader), (TAG, msg.clone()));
        assert_eq!(read_frame(&mut reader), (TAG, msg.clone()));
        assert_eq!(read_frame(&mut reader), (TAG, msg.clone()));
    }

}
