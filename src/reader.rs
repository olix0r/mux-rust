use std::io::{IoResult, IoError, Reader, InvalidInput};

use misc::{Context, Dtab, Dentry, Trace};
use proto::{types, Tag, Message,
            Treq, RreqOk, RreqError, RreqNack,
            Tdispatch, RdispatchOk, RdispatchError, RdispatchNack,
            Tdrain, Rdrain,
            Tping, Rping,
            Tdiscarded,
            Tlease,
            Rerr};

struct TraceId(u64, u64, u64);

pub trait MessageReader : Reader {

    fn read_len_vec<T>(
        &mut self,
        len: uint,
        f: |&mut Self, uint| -> IoResult<T>
     ) -> IoResult<Vec<T>> {
        let mut vec = Vec::with_capacity(len);
        for i in range(0, len) {
            match f(self, i) {
                Err(ioe) => return Err(ioe),
                Ok(t) => vec.push(t),
            }
        }
        Ok(vec)
    }


    fn read_len_buf(&mut self) -> IoResult<Vec<u8>> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe),
            Ok(len) => self.read_exact(len as uint)
        }
    }

    fn read_len_string(&mut self) -> IoResult<String> {
        match self.read_len_buf() {
            Err(ioe) => Err(ioe),
            Ok(buf) => String::from_utf8(buf).map_err(|_| -> IoError {
                IoError {
                    kind: InvalidInput,
                    desc: "invalid string",
                    detail: None,
                }
            })
        }
    }

    fn read_context(&mut self) -> IoResult<Context> {
        match self.read_len_buf() {
            Err(ioe) => Err(ioe),
            Ok(key) => match self.read_len_buf() {
                Err(ioe) => Err(ioe),
                Ok(val) => Ok(Context { key: key, val: val })
            }
        }
    }

    fn read_contexts(&mut self) -> IoResult<Vec<Context>> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe),
            Ok(len) => self.read_len_vec(len as uint, |r, _| -> IoResult<Context> {
                r.read_context()
            })
        }
    }

    fn read_dentry(&mut self) -> IoResult<Dentry> {
        match self.read_len_string() {
            Err(ioe) => Err(ioe),
            Ok(src) => match self.read_len_string() {
                Err(ioe) => Err(ioe),
                Ok(tree) => Ok(Dentry { src: src, tree: tree })
            }
        }
    }

    fn read_dtab(&mut self) -> IoResult<Dtab> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe),
            Ok(len) => self.read_len_vec(len as uint, |r, _| -> IoResult<Dentry> {
                r.read_dentry()
            }).map(|dentries| -> Dtab { Dtab(dentries) })
        }
    }

    fn read_tag(&mut self) -> IoResult<Tag> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(a) => match self.read_u8() {
                Err(ioe) => Err(ioe),
                Ok(b) => self.read_u8().map(|c| -> Tag { Tag(a, b, c) })
            }
        }
    }

    fn read_trace(&mut self) -> IoResult<Option<Trace>> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(nkeys) => {
                let mut curr_trace: Option<TraceId> = None;
                let mut curr_flags: u8 = 0;

                for _ in range(0, nkeys) {
                    match self.read_u8() {
                        Err(ioe) => return Err(ioe),
                        Ok(key) => match self.read_u8() {
                            Err(ioe) => return Err(ioe),
                            Ok(vsize) => match (key, vsize) {
                                (1, 24) => match self.read_be_u64() {
                                    Err(ioe) => return Err(ioe),
                                    Ok(span_id) => match self.read_be_u64() {
                                        Err(ioe) => return Err(ioe),
                                        Ok(parent_id) => match self.read_be_u64() {
                                            Err(ioe) => return Err(ioe),
                                            Ok(trace_id) => {
                                                let id = TraceId(span_id, parent_id, trace_id);
                                                curr_trace = Some(id)
                                            }
                                        }
                                    }
                                },
                                (2, vsize) => match self.read_exact(vsize as uint) {
                                    Err(ioe) => return Err(ioe),
                                    Ok(bytes) => match bytes.last() {
                                        // let the error be handled by a subsequent read...
                                        None => (),
                                        Some(byte) => {
                                            curr_flags = *byte
                                        }
                                    }
                                },
                                (status, vsize) => return Err(IoError {
                                    kind: InvalidInput,
                                    desc: "unknown key",
                                    detail: None,
                                })
                            }
                        }
                    }
                }

                let trace = curr_trace.map(|TraceId(span, parent, trace)| -> Trace {
                    Trace {
                        span_id: span,
                        parent_id: parent,
                        trace_id: trace,
                        flags: curr_flags,
                    }
                });
                Ok(trace)
            }
        }
    }

    fn read_treq(&mut self, tag: Tag) -> IoResult<Message> {
        match self.read_trace() {
            Err(ioe) => Err(ioe),
            Ok(trace) => self.read_to_end().map(|bytes| -> Message {
                Treq(tag, trace, bytes)
            })
        }
    }

    fn read_rreq(&mut self, tag: Tag) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(0) => self.read_to_end().map(|buf| -> Message { RreqOk(tag, buf) }),
            Ok(1) => self.read_to_string().map(|msg| -> Message { RreqError(tag, msg) }),
            Ok(2) => Ok(RreqNack(tag)),
            Ok(_) => Err(IoError {
                kind: InvalidInput,
                desc: "unknown rreq status",
                detail: None,
            })
        }
    }

    fn read_tdispatch(&mut self, tag: Tag) -> IoResult<Message> {
        match self.read_contexts() {
            Err(ioe) => Err(ioe),
            Ok(contexts) => match self.read_len_string() {
                Err(ioe) => Err(ioe),
                Ok(dst) => match self.read_dtab() {
                    Err(ioe) => Err(ioe),
                    Ok(dtab) => match self.read_to_end() {
                        Err(ioe) => Err(ioe),
                        Ok(body) => Ok(Tdispatch(tag, contexts, dst, dtab, body)),
                    }
                }
            }
        }
    }

    fn read_rdispatch(&mut self, tag: Tag) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(status) => match self.read_contexts() {
                Err(ioe) => Err(ioe),
                Ok(contexts) => match status {
                    0 => match self.read_to_end() {
                        Err(ioe) => Err(ioe),
                        Ok(body) => Ok(RdispatchOk(tag, contexts, body)),
                    },

                    1 => match self.read_to_string() {
                        Err(ioe) => Err(ioe),
                        Ok(desc) => Ok(RdispatchError(tag, contexts, desc)),
                    },

                    2 => Ok(RdispatchNack(tag, contexts)),

                    _ => Err(IoError {
                        kind: InvalidInput,
                        desc: "unknown rdispatch status",
                        detail: None,
                    })
                }
            }
        }
    }

    fn read_tdiscarded(&mut self) -> IoResult<Message> {
        match self.read_tag() {
            Err(ioe) => Err(ioe),
            Ok(which) => self.read_to_string().map(|msg| -> Message { Tdiscarded(which, msg) })
        }
    }

    fn read_tlease(&mut self) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(unit) => self.read_be_u64().map(|val| -> Message { Tlease(unit, val) })
        }
    }

    fn read_message(&mut self) -> IoResult<Message> {
        match self.read_i8() {
            Err(ioe) => Err(ioe),
            Ok(msg_type) => match self.read_tag() {
                Err(ioe) => Err(ioe),
                Ok(tag) => match msg_type {
                    types::TREQ => self.read_treq(tag),
                    types::RREQ => self.read_rreq(tag),

                    types::TDISPATCH => self.read_tdispatch(tag),
                    types::RDISPATCH => self.read_rdispatch(tag),

                    types::TDRAIN => Ok(Tdrain(tag)),
                    types::RDRAIN => Ok(Rdrain(tag)),

                    types::TPING => Ok(Tping(tag)),
                    types::RPING => Ok(Rping(tag)),

                    types::TDISCARDED => self.read_tdiscarded(),

                    types::TLEASE => self.read_tlease(),

                    types::RERR =>
                        self.read_to_string().map(|msg| -> Message { Rerr(tag, msg) }),

                    _ => Err(IoError {
                        kind: InvalidInput,
                        desc: "unknown message type",
                        detail: None,
                    })
                }
            }
        }
    }
}

impl<R: Reader> MessageReader for R {}



#[cfg(test)]
mod test {
    extern crate test;

    use std::io::{BufReader, MemWriter, IoResult};

    use proto::Tag;
    use super::MessageReader;

    fn mk_reader(bytes: &[u8]) -> BufReader { BufReader::new(bytes) }

    fn mk_str_buf(n: uint, s: &str) -> Vec<u8> {
        let mut w = MemWriter::new();
        w.write_be_u16(n as u16).ok();
        w.write_str(s).ok();
        w.unwrap()
    }

    #[test]
    fn test_tag() {
        match mk_reader([23, 45, 77, 88]).read_tag() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(tag) => assert_eq!(tag, Tag(23, 45, 77))
        }
    }

    #[test]
    fn test_len_buf() {
        match mk_reader([0, 3, 4, 5, 6, 7]).read_len_buf() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(buf) => assert_eq!(buf, vec![4, 5, 6])
        }

        match mk_reader([0, 3, 4, 5]).read_len_buf() {
            Ok(_) => fail!("did not underflow"),
            Err(_) => (),
        }

        match mk_reader([0, 0, 4, 5]).read_len_buf() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(buf) => assert_eq!(buf, vec![])
        }
    }

    #[test]
    fn test_len_string() {
        match mk_reader(mk_str_buf(3, "mom").as_slice()).read_len_string() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(s) => assert_eq!(s.as_slice(), "mom")
        }

        match mk_reader(mk_str_buf(3, "mo").as_slice()).read_len_string() {
            Ok(_) => fail!("did not underflow"),
            Err(_) => (),
        }

        match mk_reader(mk_str_buf(0, "mom").as_slice()).read_len_string() {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(s) => assert_eq!(s.as_slice(), "")
        }
    }

    static VEC_BUF: &'static [u8] = [
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
        0, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 1, 0, 0, 0];

    #[test]
    fn test_len_vec() {
        fn read_u64_vec(n: uint) -> IoResult<Vec<u64>> {
            mk_reader(VEC_BUF).read_len_vec(n, |r, _| -> IoResult<u64> {
                r.read_be_u64()
            })
        }

        match read_u64_vec(2) {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(s) => assert_eq!(s, vec![0, 256])
        }

        match read_u64_vec(5) {
            Ok(_) => fail!("did not underflow"),
            Err(_) => (),
        }

        match read_u64_vec(0) {
            Err(ioe) => fail!("read error: {}", ioe),
            Ok(s) => assert_eq!(s, vec![])
        }
    }
}
