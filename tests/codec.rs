extern crate mux;

use std::clone::Clone;
use std::old_io::{Reader, Writer, BufReader, MemWriter};

use mux::{MuxReader, MuxWriter};
use mux::misc::{Context, Dentry, Dtab};

static TDISPATCH_BUF: &'static [u8] = &[
    0, 0, 0, 65, // frame size

    2, // TDISPATCH
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

static TAG: mux::Tag = mux::Tag(4,7,9);

#[test]
fn codec_tdispatch() {
    let msg = &mux::Tmsg::Dispatch(
        vec![Context::new(vec![1,2,3,4], vec![6,7]),
             Context::new(vec![3,4], vec![6,7,8])],
        "/BAD".to_string(),
        Dtab(vec![Dentry::new("/BAD".to_string(), "/DAD".to_string())]),
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);

    /* reader */ {
        let mut buf = Vec::with_capacity(TDISPATCH_BUF.len() * 2);
        buf.extend(TDISPATCH_BUF.iter().map(|&b| -> u8 { b }));
        buf.extend(TDISPATCH_BUF.iter().map(|&b| -> u8 { b }));

        let mut reader = BufReader::new(buf.as_slice());
        let (t0, m0) = reader.read_mux_framed_tmsg().unwrap();
        assert_eq!(t0, TAG);
        assert_eq!(m0, *msg);
        let (t1, m1) = reader.read_mux_framed_tmsg().unwrap();
        assert_eq!(t1, TAG);
        assert_eq!(m1, *msg);
    }

    /* writer */ {
        let mut writer = MemWriter::new();
        writer.write_mux_framed_tmsg(&TAG, msg).ok();
        writer.write_mux_framed_tmsg(&TAG, msg).ok();
        writer.write_mux_framed_tmsg(&TAG, msg).ok();
        let buf = writer.into_inner();

        let mut reader = BufReader::new(buf.as_slice());
        let (t0, m0) = reader.read_mux_framed_tmsg().unwrap();
        assert_eq!(t0, TAG);
        assert_eq!(m0, *msg);

        let (t1, m1) = reader.read_mux_framed_tmsg().unwrap();
        assert_eq!(t1, TAG);
        assert_eq!(m1, *msg);

        let (t2, m2) = reader.read_mux_framed_tmsg().unwrap();
        assert_eq!(t2, TAG);
        assert_eq!(m2, *msg);
    }

}

#[test]
fn codec_rdispatch() {
    let msg = &mux::Rmsg::DispatchOk(Vec::new(), b"nope".to_vec());
    let tag = mux::Tag(1, 2, 3);

    let mut writer = MemWriter::new();
    writer.write_mux_framed_rmsg(&tag, msg).ok();
    let bytes = writer.into_inner();
    let expected = vec![
        0x00, 0x00, 0x00, 0x0b, // frame
        0xfe, // msg type: rdispatch (-2)
        0x01, 0x02, 0x03, // tag
        0x00, // status
        0x00, 0x00, // contexts
        0x6e, 0x6f, 0x70, 0x65 // "nope""
        ];
    assert_eq!(bytes, expected);

    {
        let mut reader = BufReader::new(bytes.as_slice().slice_from(8));
        assert_eq!(reader.read_mux_contexts().unwrap(), vec![]);
    }

    {
        let mut reader = BufReader::new(bytes.as_slice().slice_from(4));
        let (t, m) = reader.read_mux_rmsg().unwrap();
        assert_eq!(t, tag);
        assert_eq!(m, *msg);
    }

    {
        let mut reader = BufReader::new(bytes.as_slice());
        let (t, m) = reader.read_mux_framed_rmsg().unwrap();
        assert_eq!(t, tag);
        assert_eq!(m, *msg);
    }
}
