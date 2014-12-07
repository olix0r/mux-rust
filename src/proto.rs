use misc::{Context, Dtab, Trace};

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Tag(pub u8, pub u8, pub u8);
pub static MARKER_TAG: Tag = Tag(0,0,0);

#[deriving(Clone,PartialEq,Eq,Show)]
pub enum Message {
    Treq(Tag, Option<Trace>, Vec<u8>),
    RreqOk(Tag, Vec<u8>),
    RreqError(Tag, String),
    RreqNack(Tag),

    Tdispatch(Tag, Vec<Context>, String, Dtab, Vec<u8>),
    RdispatchOk(Tag, Vec<Context>, Vec<u8>),
    RdispatchError(Tag, Vec<Context>, String),
    RdispatchNack(Tag, Vec<Context>),

    Tdrain(Tag),
    Rdrain(Tag),

    Tping(Tag),
    Rping(Tag),

    Tdiscarded(Tag, String),

    Tlease(u8, u64),

    Rerr(Tag, String),
}

pub mod types {
    pub static TREQ: i8 =  1;
    pub static RREQ: i8 = -1;

    pub static TDISPATCH: i8 =  2;
    pub static RDISPATCH: i8 = -2;

    pub static TDRAIN: i8 =  64;
    pub static RDRAIN: i8 = -64;

    pub static TPING: i8 =  65;
    pub static RPING: i8 = -65;

    pub static TDISCARDED: i8 = 66;

    pub static TLEASE: i8 = 67;

    pub static RERR: i8 = -128;
}

#[cfg(test)]
mod test {
    use misc::{Context, Dentry, Dtab};
    use reader::MessageReader;
    use writer::MessageWriter;
    use super::{Message, Tag, Treq, Tdispatch, Tdrain, Tping, Tdiscarded, Tlease};
    use std::io::{BufReader, MemWriter};

    fn assert_encode(msg: &Message) -> Vec<u8> {
        let mut writer = MemWriter::new();
        match writer.write_message(msg) {
            Err(ioe) => fail!("write error: {}", ioe),
            Ok(_) => writer.unwrap()
        }
    }

    fn assert_decode(bytes: Vec<u8>) -> Message {
        let mut reader = BufReader::new(bytes.as_slice());
        match reader.read_message() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(decoded) => decoded
        }
    }

    fn assert_decode_encoded(len: uint, msg: &Message) {
        let bytes = assert_encode(msg);
        assert_eq!(bytes.len(), len);

        let decoded = assert_decode(bytes);
        assert_eq!(*msg, decoded);
    }

    #[test]
    fn test_decode_treq() {
        let mut sz = 1;

        let tag = Tag(0,0,1);
        sz += 3;

        let trace = None;
        sz += 1;

        let body = b"momma";
        sz += 5;

        assert_decode_encoded(sz, &Treq(tag, trace, body.to_vec()));
    }

    #[test]
    fn test_decode_tdispatch() {
        let mut sz = 1;

        let tag = Tag(0,0,1);
        sz += 3;

        let contexts = vec![Context::new(vec![1], vec![2, 3])];
        sz += 2 + 2+1 + 2+2;

        let dst = "/ugh".to_string();
        sz += 2+4;

        let dtab = Dtab(vec![Dentry::new("/foo".to_string(), "/bars".to_string())]);
        sz += 2 + 2+4 + 2+5;

        let body = b"mom".to_vec();
        sz += 3;

        assert_decode_encoded(sz, &Tdispatch(tag, contexts, dst, dtab, body));
    }

    #[test]
    fn test_decode_tdrain() {
        assert_decode_encoded(4, &Tdrain(Tag(0,0,1)));
    }

    #[test]
    fn test_decode_tping() {
        assert_decode_encoded(4, &Tping(Tag(0,0,1)));
    }

    #[test]
    fn test_decode_tdiscarded() {
        assert_decode_encoded(4 + 3 + 3, &Tdiscarded(Tag(0,1,0), "msg".to_string()));
    }

    #[test]
    fn test_decode_tlease() {
        assert_decode_encoded(4 + 1 + 8, &Tlease(60, 30));
    }
}
