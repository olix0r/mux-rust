use std::io::{IoResult, IoError, BufReader};

use misc::{Context, Path, Dtab};

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Tag(u64);

#[deriving(Clone,PartialEq,Eq,Show)]
pub enum Message {
    // Treq(Tag, Option<TraceId>, Buf),
    // RreqOk(Tag, Buf),
    // RreqError(Tag, str),
    //RreqNack(Tag),

    Tdispatch(Tag, Vec<Context>, Path, Dtab, Vec<u8>),
    RdispatchOk(Tag, Vec<u8>),
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

#[deriving(Clone,PartialEq,Eq,Show)]
pub enum MessageErr {
    IoErr(IoError),
    UnknownType(i8),
}

pub type Decoded<T> = Result<T, MessageErr>;

trait MessageReader : Reader {
    fn read_length_encoded<T>(&mut self, f: |&mut Self, uint| -> IoResult<T>) -> IoResult<Vec<T>>;
    fn read_buf(&mut self) -> IoResult<Vec<u8>>;
    fn read_context(&mut self) -> IoResult<Context>;
    fn read_contexts(&mut self) -> IoResult<Vec<Context>>;
    fn read_tdispatch(&mut self, tag: Tag) -> IoResult<Message>;
    fn read_tag(&mut self) -> IoResult<Tag>;
}

impl<'t> MessageReader for BufReader<'t> {
    fn read_length_encoded<T>(&mut self, f: |&mut BufReader<'t>, uint| -> IoResult<T>) -> IoResult<Vec<T>> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe),
            Ok(_len) => {
                let len = _len as uint;
                let mut vec = Vec::with_capacity(len);
                for i in range(0, len) {
                    match f(self, i) {
                        Err(ioe) => return Err(ioe),
                        Ok(t) => vec.push(t),
                    }
                }
                Ok(vec)
            }
        }
    }

    fn read_buf(&mut self) -> IoResult<Vec<u8>> {
        self.read_length_encoded(|r, _| -> IoResult<u8> { r.read_u8() })
    }

    fn read_context(&mut self) -> IoResult<Context> {
        match self.read_buf() {
            Err(ioe) => Err(ioe),
            Ok(key) => match self.read_buf() {
                Err(ioe) => Err(ioe),
                Ok(val) => Ok(Context { key: key, val: val })
            }
        }
    }

    fn read_contexts(&mut self) -> IoResult<Vec<Context>> {
        self.read_length_encoded(|r, _| -> IoResult<Context> { r.read_context() })
    }

    fn read_tdispatch(&mut self, tag: Tag) -> IoResult<Message> {
        match self.read_contexts() {
            Err(ioe) => Err(ioe),
            Ok(_) => match self.read_buf() {
                Err(ioe) => Err(ioe),
                Ok(_) => {
                    // todo
                    Ok(Rerr(tag, "fuckit".to_string()))
                }
            }
        }
    }

    fn read_tag(&mut self) -> IoResult<Tag> {
        self.read_be_uint_n(3).map(|t| -> Tag { Tag(t) })
    }
}

impl Message {
    pub fn decode(bytes: &[u8]) -> Decoded<Message> {
        let mut reader = BufReader::new(bytes);
        match reader.read_i8() {
            Err(ioe) => Err(IoErr(ioe)),
            Ok(msg_type) => match reader.read_tag() {
                Err(ioe) => Err(IoErr(ioe)),
                Ok(tag) => match msg_type {
                    2 => match reader.read_tdispatch(tag) {
                        Err(ioe) => Err(IoErr(ioe)),
                        Ok(msg) => Ok(msg),
                    },
                    //-1 => self.read_Rdispatch(reader, tag),
                    t => Err(UnknownType(t))
                }
            }
        }
    }
}
