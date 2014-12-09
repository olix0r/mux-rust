use std::io::{IoResult, IoError, Reader, InvalidInput, BufReader};

use misc::{Context, Dtab, Dentry, Trace, Detailed};
use proto::{types, Tag, Message,
            Treq, RreqOk, RreqError, RreqNack,
            Tdispatch, RdispatchOk, RdispatchError, RdispatchNack,
            Tdrain, Rdrain,
            Tping, Rping,
            Tdiscarded,
            Tlease,
            Rerr};

struct TraceId(u64, u64, u64);

pub trait FrameReader: Reader {
    fn read_be_u32_frame(&mut self) -> IoResult<Vec<u8>> {
        match self.read_be_u32() {
            Err(ioe) => Err(ioe.detail("frame size")),
            Ok(sz) => match self.read_exact(sz as uint) {
                Err(ioe) => Err(ioe.detail("frame body")),
                ok => ok
            }
        }
    }
}

impl<R: Reader> FrameReader for R {}

pub trait MuxReader: FrameReader {

    fn read_mux_frame(&mut self) -> IoResult<(Tag, Message)> {
        match self.read_be_u32_frame() {
            Err(ioe) => Err(ioe),
            Ok(bytes) => {
                // println!("read frame: {}",
                //          bytes.iter().fold(String::new(), |s,&b| -> String {
                //              format!("{}{:02x}", s, b)
                //          }));
                let mut buf = BufReader::new(bytes.as_slice());
                buf.read_mux()
            },
        }
    }

    fn read_mux(&mut self) -> IoResult<(Tag, Message)> {
        match self.read_i8() {
            Err(ioe) => Err(ioe),

            Ok(typ) => match types::Message::from_i8(typ) {
                None => Err(IoError {
                    kind: InvalidInput,
                    desc: "unknown message type",
                    detail: Some(format!("{}", typ)),
                }),

                Some(typ) => match self.read_mux_tag() {
                    Err(ioe) => Err(ioe),

                    Ok(tag) => match self.read_mux_message(typ) {
                        Err(ioe) => Err(ioe),
                        Ok(msg) => Ok((tag, msg))
                    }
                }
            }
        }
    }

    fn read_mux_message(&mut self, msg_type: types::Message) -> IoResult<Message> {
        match msg_type {
            types::Treq => self.read_mux_treq(),
            types::Rreq => self.read_mux_rreq(),

            types::Tdispatch => self.read_mux_tdispatch(),
            types::Rdispatch => self.read_mux_rdispatch(),

            types::Tdrain => Ok(Tdrain),
            types::Rdrain => Ok(Rdrain),

            types::Tping => Ok(Tping),
            types::Rping => Ok(Rping),

            types::Tdiscarded => self.read_mux_tdiscarded(),

            types::Tlease => self.read_mux_tlease(),

            types::Rerr => self.read_to_string().map(|msg| -> Message { Rerr(msg) }),
        }
    }

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
            Err(ioe) => Err(ioe.detail("in buf")),
            Ok(len) => self.read_exact(len as uint)
        }
    }

    fn read_len_string(&mut self) -> IoResult<String> {
        match self.read_len_buf() {
            Err(ioe) => Err(ioe),
            Ok(buf) => String::from_utf8(buf).map_err(|_| -> IoError {
                IoError {
                    kind: InvalidInput,
                    desc: "not a utf8 string",
                    detail: None,
                }
            })
        }
    }

    fn read_mux_context(&mut self) -> IoResult<Context> {
        match self.read_len_buf() {
            Err(ioe) => Err(ioe.detail("in context: key")),
            Ok(key) => match self.read_len_buf() {
                Err(ioe) => Err(ioe.detail("in context: val")),
                Ok(val) => Ok(Context { key: key, val: val })
            }
        }
    }

    fn read_mux_contexts(&mut self) -> IoResult<Vec<Context>> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe.detail("in contexts")),
            Ok(len) => self.read_len_vec(len as uint, |r, _| -> IoResult<Context> {
                r.read_mux_context()
            })
        }
    }

    fn read_mux_dentry(&mut self) -> IoResult<Dentry> {
        match self.read_len_string() {
            Err(ioe) => Err(ioe.detail("in dentry")),
            Ok(src) => match self.read_len_string() {
                Err(ioe) => Err(ioe.detail("in dentry")),
                Ok(tree) => Ok(Dentry { src: src, tree: tree })
            }
        }
    }

    fn read_mux_dtab(&mut self) -> IoResult<Dtab> {
        match self.read_be_u16() {
            Err(ioe) => Err(ioe.detail("in dtab")),
            Ok(len) => self.read_len_vec(len as uint, |r, _| -> IoResult<Dentry> {
                r.read_mux_dentry()
            }).map(|dentries| -> Dtab { Dtab(dentries) })
        }
    }

    fn read_mux_tag(&mut self) -> IoResult<Tag> {
        match self.read_u8() {
            Err(ioe) => Err(ioe.detail("in tag")),
            Ok(a) => match self.read_u8() {
                Err(ioe) => Err(ioe.detail("in tag")),
                Ok(b) => match self.read_u8() {
                    Err(ioe) => Err(ioe.detail("in tag")),
                    Ok(c) => Ok(Tag(a, b, c))
                }
            }
        }
    }

    fn read_mux_trace(&mut self) -> IoResult<Option<Trace>> {
        match self.read_u8() {
            Err(ioe) => Err(ioe.detail("in trace: nkeys")),
            Ok(nkeys) => {
                let mut curr_trace: Option<TraceId> = None;
                let mut curr_flags: u8 = 0;

                for _ in range(0, nkeys) {
                    match self.read_u8() {
                        Err(ioe) =>
                            return Err(ioe.detail("in trace: key")),

                        Ok(key) => match self.read_u8() {
                            Err(ioe) =>
                                return Err(ioe.detail("in trace")),

                            Ok(vsize) => match (key, vsize) {
                                (1, 24) => match self.read_be_u64() {
                                    Err(ioe) =>
                                        return Err(ioe.detail("in trace")),

                                    Ok(span_id) => match self.read_be_u64() {
                                        Err(ioe) =>
                                            return Err(ioe.detail("in trace")),

                                        Ok(parent_id) => match self.read_be_u64() {
                                            Err(ioe) =>
                                                return Err(ioe.detail("in trace")),

                                            Ok(trace_id) => {
                                                let id = TraceId(span_id, parent_id, trace_id);
                                                curr_trace = Some(id)
                                            }
                                        }
                                    }
                                },

                                (2, vsize) => match self.read_exact(vsize as uint) {
                                    Err(ioe) => return Err(ioe.detail("in trace")),

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

    fn read_mux_treq(&mut self) -> IoResult<Message> {
        match self.read_mux_trace() {
            Err(ioe) => Err(ioe),

            Ok(trace) => match self.read_to_end() {
                Err(ioe) => Err(ioe.detail("(in treq: body)")),

                Ok(bytes) => Ok(Treq(trace, bytes))
            }
        }
    }

    fn read_mux_rreq(&mut self) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(0) => self.read_to_end().map(|buf| -> Message { RreqOk(buf) }),
            Ok(1) => self.read_to_string().map(|msg| -> Message { RreqError(msg) }),
            Ok(2) => Ok(RreqNack),
            Ok(_) => Err(IoError {
                kind: InvalidInput,
                desc: "unknown rreq status",
                detail: None,
            })
        }
    }

    fn read_mux_tdispatch(&mut self) -> IoResult<Message> {
        match self.read_mux_contexts() {
            Err(ioe) => Err(ioe),

            Ok(contexts) => match self.read_len_string() {
                Err(ioe) => Err(ioe),

                Ok(dst) => match self.read_mux_dtab() {
                    Err(ioe) => Err(ioe),

                    Ok(dtab) => match self.read_to_end() {
                        Err(ioe) => Err(ioe),

                        Ok(body) => Ok(Tdispatch(contexts, dst, dtab, body)),
                    }
                }
            }
        }
    }

    fn read_mux_rdispatch(&mut self) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(status) => match self.read_mux_contexts() {
                Err(ioe) => Err(ioe),
                Ok(contexts) => match status {
                    0 => match self.read_to_end() {
                        Err(ioe) => Err(ioe),
                        Ok(body) => Ok(RdispatchOk(contexts, body)),
                    },

                    1 => match self.read_to_string() {
                        Err(ioe) => Err(ioe),
                        Ok(desc) => Ok(RdispatchError(contexts, desc)),
                    },

                    2 => Ok(RdispatchNack(contexts)),

                    _ => Err(IoError {
                        kind: InvalidInput,
                        desc: "unknown rdispatch status",
                        detail: None,
                    })
                }
            }
        }
    }

    fn read_mux_tdiscarded(&mut self) -> IoResult<Message> {
        match self.read_mux_tag() {
            Err(ioe) => Err(ioe),
            Ok(which) => self.read_to_string().map(|msg| -> Message { Tdiscarded(which, msg) })
        }
    }

    fn read_mux_tlease(&mut self) -> IoResult<Message> {
        match self.read_u8() {
            Err(ioe) => Err(ioe),
            Ok(unit) => self.read_be_u64().map(|val| -> Message { Tlease(unit, val) })
        }
    }

}

impl<R: Reader> MuxReader for R {}

#[cfg(test)]
mod test {
    extern crate test;

    use std::io::{BufReader, MemWriter, IoResult};

    use proto::Tag;
    use super::MuxReader;

    fn mk_reader(bytes: &[u8]) -> BufReader { BufReader::new(bytes) }

    fn mk_str_buf(n: uint, s: &str) -> Vec<u8> {
        let mut w = MemWriter::new();
        w.write_be_u16(n as u16).ok();
        w.write_str(s).ok();
        w.unwrap()
    }

    #[test]
    fn test_tag() {
        let bytes = [23, 45, 77, 88];
        let mut r = BufReader::new(bytes);
        assert_eq!(r.read_mux_tag().unwrap(), Tag(23, 45, 77));
        assert_eq!(r.read_u8().unwrap(), 88);
    }

    #[test]
    fn test_contexts() {
        let bytes = [0x00, 0x00, // contexts
                     0x6e, 0x6f, 0x70, 0x65]; // "nope""
        let mut r = BufReader::new(bytes);
        assert_eq!(r.read_mux_contexts().unwrap(), vec![]);
        assert_eq!(r.read_u8().unwrap(), 0x6e);
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
