use misc::{Context, Dtab, Trace};

#[derive(Clone,PartialEq,Eq,Show,Copy)]
pub struct Tag(pub u8, pub u8, pub u8);

pub static MARKER_TAG: Tag = Tag(0,0,0);

mod types {
    pub const TREQ: i8 =  1;
    pub const RREQ: i8 = -1;

    pub const TDISPATCH: i8 =  2;
    pub const RDISPATCH: i8 = -2;

    pub const TDRAIN: i8 =  64;
    pub const RDRAIN: i8 = -64;

    pub const TPING: i8 =  65;
    pub const RPING: i8 = -65;

    pub const TDISCARDED: i8 = 66;

    pub const TLEASE: i8 = 67;

    pub const RERR: i8 = -128;
}


#[derive(Clone,Eq,PartialEq,Show)]
pub enum MsgType {
    Treq, Rreq,
    Tdispatch, Rdispatch,
    Tdrain, Rdrain,
    Tping, Rping,
    Tdiscarded,
    Tlease,
    Rerr,
}

impl MsgType {
    pub fn from_i8(code: i8) -> Option<MsgType> {
        match code {
            types::TREQ => Some(MsgType::Treq),
            types::RREQ => Some(MsgType::Rreq),

            types::TDISPATCH => Some(MsgType::Tdispatch),
            types::RDISPATCH => Some(MsgType::Rdispatch),

            types::TDRAIN => Some(MsgType::Tdrain),
            types::RDRAIN => Some(MsgType::Rdrain),

            types::TPING => Some(MsgType::Tping),
            types::RPING => Some(MsgType::Rping),

            types::TDISCARDED => Some(MsgType::Tdiscarded),

            types::TLEASE => Some(MsgType::Tlease),

            types::RERR => Some(MsgType::Rerr),

            _ => None
        }
    }

    pub fn to_i8(self) -> i8 {
        match self {
            MsgType::Treq => types::TREQ,
            MsgType::Rreq => types::RREQ,

            MsgType::Tdispatch => types::TDISPATCH,
            MsgType::Rdispatch => types::RDISPATCH,

            MsgType::Tping => types::TPING,
            MsgType::Rping => types::RPING,

            MsgType::Tdrain => types::TDRAIN,
            MsgType::Rdrain => types::RDRAIN,

            MsgType::Tdiscarded => types::TDISCARDED,

            MsgType::Tlease => types::TLEASE,

            MsgType::Rerr => types::RERR,
        }
    }
}

#[derive(Clone,Eq,PartialEq,Show)]
pub enum Tmsg {
    Req(Option<Trace>, Vec<u8>),
    Dispatch(Vec<Context>, String, Dtab, Vec<u8>),
    Drain,
    Ping,
    Discarded(Tag, String),
    Lease(u8, u64),
}


impl Tmsg {
    pub fn get_type(&self) -> MsgType {
        match self {
            &Tmsg::Req(_, _)   => MsgType::Treq,
            &Tmsg::Dispatch(_, _, _, _) => MsgType::Tdispatch,
            &Tmsg::Drain => MsgType::Tdrain,
            &Tmsg::Ping => MsgType::Tping,
            &Tmsg::Discarded(_, _) => MsgType::Tdiscarded,
            &Tmsg::Lease(_, _) => MsgType::Tlease,
        }
    }
}

#[derive(Clone,Eq,PartialEq,Show)]
pub enum Rmsg {
    ReqOk(Vec<u8>),
    ReqError(String),
    ReqNack,

    DispatchOk(Vec<Context>, Vec<u8>),
    DispatchError(Vec<Context>, String),
    DispatchNack(Vec<Context>),

    Drain,
    Ping,

    Err(String),
}

impl Rmsg {
    pub fn get_type(&self) -> MsgType {
        match self {
            &Rmsg::ReqOk(_)    => MsgType::Rreq,
            &Rmsg::ReqError(_) => MsgType::Rreq,
            &Rmsg::ReqNack     => MsgType::Rreq,

            &Rmsg::DispatchOk(_, _)     => MsgType::Rdispatch,
            &Rmsg::DispatchError(_, _)  => MsgType::Rdispatch,
            &Rmsg::DispatchNack(_)      => MsgType::Rdispatch,

            &Rmsg::Drain => MsgType::Rdrain,

            &Rmsg::Ping => MsgType::Rping,

            &Rmsg::Err(_) => MsgType::Rerr,
        }
    }
}

#[cfg(test)]
mod test {
    use misc::{Context, Dentry, Dtab};
    use std::io::{Reader, BufReader, MemWriter};
    use reader::MuxReader;
    use writer::MuxWriter;
    use super::{MsgType, Tmsg, Tag};

    fn assert_encode(msg: &Tmsg) -> Vec<u8> {
        let mut writer = MemWriter::new();
        writer.write_mux_tmsg_msg(msg).unwrap();
        writer.into_inner()
    }

    fn assert_decode(t: MsgType, bytes: Vec<u8>) -> Tmsg {
        let mut reader = BufReader::new(bytes.as_slice());
        reader.read_mux_tmsg_msg(t).unwrap()
    }

    fn assert_decode_encoded(len: usize, msg: &Tmsg) {
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

        assert_decode_encoded(sz, &Tmsg::Req(trace, body.to_vec()));
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

        assert_decode_encoded(sz, &Tmsg::Dispatch(contexts, dst, dtab, body));
    }

    #[test]
    fn test_decode_tdrain() {
        assert_decode_encoded(0, &Tmsg::Drain);
    }

    #[test]
    fn test_decode_tping() {
        assert_decode_encoded(0, &Tmsg::Ping);
    }

    #[test]
    fn test_decode_tdiscarded() {
        assert_decode_encoded(3 + 3, &Tmsg::Discarded(Tag(0,1,0), "msg".to_string()));
    }

    #[test]
    fn test_decode_tlease() {
        assert_decode_encoded(1 + 8, &Tmsg::Lease(60, 30));
    }
}
