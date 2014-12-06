use std::io::{IoResult, IoError, Reader, Writer, InvalidInput};

use misc::{Context, Dtab, Dentry, Trace};

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Tag(pub u8, pub u8, pub u8);
static MARKER_TAG: Tag = Tag(0,0,0);

struct TraceId(u64, u64, u64);

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
    Tdiscarded(u64, String),
    Tlease(u8, u64),

    Rerr(Tag, String),
}

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
                                            Ok(trace_id) =>
                                                curr_trace = Some(TraceId(span_id, parent_id, trace_id)),
                                        }
                                    }
                                },
                                (2, vsize) => match self.read_exact(vsize as uint) {
                                    Err(ioe) => return Err(ioe),
                                    Ok(bytes) => match bytes.last() {
                                        None => (), // let the error be handled by a subsequent read...
                                        Some(byte) =>
                                            curr_flags = *byte,
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
        match self.read_be_uint_n(3) {
            Err(ioe) => Err(ioe),
            Ok(which) => self.read_to_string().map(|msg| -> Message {
                Tdiscarded(which, msg)
            })
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
                     1 => self.read_treq(tag),
                    -1 => self.read_rreq(tag),

                     2 => self.read_tdispatch(tag),
                    -2 => self.read_rdispatch(tag),

                     64 => Ok(Tdrain(tag)),
                    -64 => Ok(Rdrain(tag)),

                     65 => Ok(Tping(tag)),
                    -65 => Ok(Rping(tag)),

                     66 => self.read_tdiscarded(),
                     67 => self.read_tlease(),

                    -128 => self.read_to_string().map(|msg| -> Message { Rerr(tag, msg) }),

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

pub trait MessageWriter : Writer {
    fn write_len_vec<T>(&mut self, ts: &[T], f: |&mut Self, &T| -> IoResult<()>) -> IoResult<()> {
        match self.write_be_u16(ts.len() as u16) {
            Err(ioe) => Err(ioe),
            Ok(_) => {
                for t in ts.iter() {
                    match f(self, t) {
                        Err(ioe) => return Err(ioe),
                        Ok(_) => (),
                    }
                }
                Ok(())
            }
        }
    }

    fn write_len_buf(&mut self, buf: &[u8]) -> IoResult<()> {
        match self.write_be_u16(buf.len() as u16) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write(buf)
        }
    }

    fn write_len_str(&mut self, s: &str) -> IoResult<()> {
        match self.write_be_u16(s.len() as u16) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_str(s)
        }
    }

    fn write_context(&mut self, context: &Context) -> IoResult<()> {
        match self.write_len_buf(context.key.as_slice()) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_len_buf(context.val.as_slice())
        }
    }

    fn write_contexts(&mut self, contexts: &[Context]) -> IoResult<()> {
        self.write_len_vec(contexts, |w, ctx| -> IoResult<()> { w.write_context(ctx) })
    }

    fn write_dentry(&mut self, dentry: &Dentry) -> IoResult<()> {
        match self.write_len_str(dentry.src.as_slice()) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_len_str(dentry.tree.as_slice())
        }
    }

    fn write_dtab(&mut self, dtab: &Dtab) -> IoResult<()> {
        let &Dtab(ref dentries) = dtab;
        self.write_len_vec(dentries.as_slice(), |w, d| -> IoResult<()> { w.write_dentry(d) })
    }

    fn write_trace(&mut self, trace: Option<Trace>) -> IoResult<()> {
        match trace {
            None => self.write_u8(0),
            Some(trace) => match self.write_u8(2) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_u8(0) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_u8(24) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => match self.write_be_u64(trace.span_id) {
                            Err(ioe) => Err(ioe),
                            Ok(_) => match self.write_be_u64(trace.parent_id) {
                                Err(ioe) => Err(ioe),
                                Ok(_) => match self.write_be_u64(trace.trace_id) {
                                    Err(ioe) => Err(ioe),
                                    Ok(_) => self.write([1, 1, trace.flags])
                                }
                            }
                        }
                    }
                },
            },
        }
    }

    fn write_head(&mut self, typ: i8, tag: Tag) -> IoResult<()> {
        match self.write_i8(typ) {
            Err(ioe) => Err(ioe),
            Ok(_) => {
                let Tag(b0, b1, b2) = tag;
                self.write([b0, b1, b2])
            }
        }
    }

    fn write_message(&mut self, m: &Message) -> IoResult<()> {
        match m {
            &Treq(tag, trace, ref body) => match self.write_head(1, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_trace(trace) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                }
            },

            &RreqOk(tag, ref body) => match self.write_head(-1, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write(body.as_slice())
            },
            &RreqError(tag, ref s) => match self.write_head(-1, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write_str(s.as_slice())
            },
            &RreqNack(tag) => self.write_head(-1, tag),

            &Tdispatch(tag, ref contexts, ref dst, ref dtab, ref body) => match self.write_head(2, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_len_str(dst.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => match self.write_dtab(dtab) {
                            Err(ioe) => Err(ioe),
                            Ok(_) => self.write(body.as_slice())
                         }
                    }
                }
            },

            &RdispatchOk(tag, ref contexts, ref body) => match self.write_head(-2, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                }
            },
            &RdispatchError(tag, ref contexts, ref msg) => match self.write_head(-2, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(msg.as_slice())
                }
            },
            &RdispatchNack(tag, ref contexts) => match self.write_head(-2, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write_contexts(contexts.as_slice())
            },

            &Tdrain(tag) => self.write_head(64, tag),
            &Rdrain(tag) => self.write_head(-64, tag),

            &Tping(tag) => self.write_head(65, tag),
            &Rping(tag) => self.write_head(-65, tag),

            &Tdiscarded(which, ref msg) => match self.write_head(67, MARKER_TAG) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_be_u64(which) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(msg.as_slice())
                }
            },

            &Tlease(unit, amount) => match self.write_head(67, MARKER_TAG) {
                Err(ioe) => Err(ioe),
                Ok(_) => match self.write_u8(unit) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_be_u64(amount)
                }
            },

            &Rerr(tag, ref msg) => match self.write_head(-128, tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write_str(msg.as_slice())
            },
        }
    }
}


impl<R: Reader> MessageReader for R {}
impl<W: Writer> MessageWriter for W {}

#[cfg(test)]
mod test {
    use misc::{Context, Dentry, Dtab};
    use super::{Tag, Tdispatch, MessageReader, MessageWriter};
    use std::io::{BufReader, MemWriter};

    #[test]
    fn test_decode_encoded_tdispatch() {
        let tag = Tag(0,0,1);
        let contexts = vec![Context::new(vec![1], vec![2])];
        let dst = "/ugh".to_string();
        let dtab = Dtab(vec![Dentry::new("/foo".to_string(), "/bar".to_string())]);
        let body = vec!['m' as u8, 'o' as u8, 'm' as u8];
        let msg = Tdispatch(tag, contexts, dst, dtab, body);

        let mut writer = MemWriter::new();
        match writer.write_message(&msg) {
            Err(_) => fail!("write error"),
            Ok(_) => {
                let vec = writer.unwrap();
                let mut reader = BufReader::new(vec.as_slice());
                match reader.read_message() {
                    Err(_) => fail!("read error"),
                    Ok(decoded) => assert_eq!(msg, decoded),
                }
            }
        }
    }
}
