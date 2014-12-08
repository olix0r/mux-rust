use misc::{Context, Dtab, Trace};

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Tag(pub u8, pub u8, pub u8);
pub static MARKER_TAG: Tag = Tag(0,0,0);

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

    #[deriving(Clone,PartialEq,Eq,Show)]
    pub enum Message {
        Treq = TREQ as int,
        Rreq = RREQ as int,

        Tdispatch = TDISPATCH as int,
        Rdispatch = RDISPATCH as int,

        Tdrain = TDRAIN as int,
        Rdrain = RDRAIN as int,

        Tping = TPING as int,
        Rping = RPING as int,

        Tdiscarded = TDISCARDED as int,

        Tlease = TLEASE as int,

        Rerr = RERR as int,
    }

    impl Message {
        pub fn from_i8(code: i8) -> Option<Message> {
            match code {
                TREQ => Some(Treq),
                RREQ => Some(Rreq),
                TDISPATCH => Some(Tdispatch),
                RDISPATCH => Some(Rdispatch),
                TPING => Some(Tping),
                RPING => Some(Rping),
                TDISCARDED => Some(Tdiscarded),
                TLEASE => Some(Tlease),
                RERR => Some(Rerr),
                _ => None
            }
        }
    }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Header(pub u32, pub types::Message, pub Tag);

#[deriving(Clone,PartialEq,Eq,Show)]
pub enum Message {
    Treq(Option<Trace>, Vec<u8>),
    RreqOk(Vec<u8>),
    RreqError(String),
    RreqNack,

    Tdispatch(Vec<Context>, String, Dtab, Vec<u8>),
    RdispatchOk(Vec<Context>, Vec<u8>),
    RdispatchError(Vec<Context>, String),
    RdispatchNack(Vec<Context>),

    Tdrain,
    Rdrain,

    Tping,
    Rping,

    Tdiscarded(Tag, String),

    Tlease(u8, u64),

    Rerr(String),
}

impl Message {
    pub fn get_type(&self) -> types::Message {
        match self {
            &Treq(_, _) => types::Treq,
            &RreqOk(_) => types::Rreq,
            &RreqError(_) => types::Rreq,
            &RreqNack => types::Rreq,

            &Tdispatch(_, _, _, _) => types::Tdispatch,
            &RdispatchOk(_, _) => types::Rdispatch,
            &RdispatchError(_, _) => types::Rdispatch,
            &RdispatchNack(_) => types::Rdispatch,

            &Tdrain => types::Tdrain,
            &Rdrain => types::Rdrain,

            &Tping => types::Tping,
            &Rping => types::Rping,

            &Tdiscarded(_, _) => types::Tdiscarded,

            &Tlease(_, _) => types::Tlease,

            &Rerr(_) => types::Rerr,
        }
    }
}

pub struct Framed(pub Tag, pub Message);

#[cfg(test)]
mod test {
    use misc::{Context, Dentry, Dtab};
    use reader::MessageReader;
    use writer::MessageWriter;
    use super::{types, Message, Tag, Treq, Tdispatch, Tdrain, Tping, Tdiscarded, Tlease};
    use std::io::{Reader, BufReader, MemWriter};

    fn assert_encode(msg: &Message) -> Vec<u8> {
        let mut writer = MemWriter::new();
        match writer.write_message_body(msg) {
            Err(ioe) => fail!("write error: {}", ioe),
            Ok(_) => writer.unwrap()
        }
    }

    fn assert_decode(t: types::Message, bytes: Vec<u8>) -> Message {
        let mut reader = BufReader::new(bytes.as_slice());
        match reader.read_message_body(t) {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(decoded) => decoded
        }
    }

    fn assert_decode_encoded(len: uint, msg: &Message) {
        let bytes = assert_encode(msg);
        assert_eq!(bytes.len(), len);

        let decoded = assert_decode(msg.get_type(), bytes);
        assert_eq!(*msg, decoded);
    }

    #[test]
    fn test_decode_treq() {
        let mut sz = 0;

        let trace = None;
        sz += 1;

        let body = b"momma";
        sz += 5;

        assert_decode_encoded(sz, &Treq(trace, body.to_vec()));
    }

    #[test]
    fn test_decode_tdispatch() {
        let mut sz = 0;

        let contexts = vec![Context::new(vec![1], vec![2, 3])];
        sz += 2 + 2+1 + 2+2;

        let dst = "/ugh".to_string();
        sz += 2+4;

        let dtab = Dtab(vec![Dentry::new("/foo".to_string(), "/bars".to_string())]);
        sz += 2 + 2+4 + 2+5;

        let body = b"mom".to_vec();
        sz += 3;

        assert_decode_encoded(sz, &Tdispatch(contexts, dst, dtab, body));
    }

    #[test]
    fn test_decode_tdrain() {
        assert_decode_encoded(0, &Tdrain);
    }

    #[test]
    fn test_decode_tping() {
        assert_decode_encoded(0, &Tping);
    }

    #[test]
    fn test_decode_tdiscarded() {
        assert_decode_encoded(3 + 3, &Tdiscarded(Tag(0,1,0), "msg".to_string()));
    }

    #[test]
    fn test_decode_tlease() {
        assert_decode_encoded(1 + 8, &Tlease(60, 30));
    }
}
