use std::io::{IoResult, Writer};

use misc::{Context, Dtab, Dentry, Trace};
use proto::{types, Tag, MARKER_TAG, Message,
            Treq, RreqOk, RreqError, RreqNack,
            Tdispatch, RdispatchOk, RdispatchError, RdispatchNack,
            Tdrain, Rdrain,
            Tping, Rping,
            Tdiscarded,
            Tlease,
            Rerr};

pub trait MessageWriter : Writer {

    fn write_message(&mut self, m: &Message) -> IoResult<()> {
        match m {
            &Treq(tag, trace, ref body) =>
                match self.write_head(types::TREQ, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_trace(trace) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write(body.as_slice())
                    }
                },

            &RreqOk(tag, ref body) =>
                match self.write_head(types::RREQ, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                },
            &RreqError(tag, ref s) =>
                match self.write_head(types::RREQ, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(s.as_slice())
                },
            &RreqNack(tag) =>
                self.write_head(types::RREQ, tag),

            &Tdispatch(tag, ref contexts, ref dst, ref dtab, ref body) =>
                match self.write_head(types::TDISPATCH, tag) {
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

            &RdispatchOk(tag, ref contexts, ref body) =>
                match self.write_head(types::RDISPATCH, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_contexts(contexts.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write(body.as_slice())
                    }
                },
            &RdispatchError(tag, ref contexts, ref msg) =>
                match self.write_head(types::RDISPATCH, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_contexts(contexts.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write_str(msg.as_slice())
                    }
                },
            &RdispatchNack(tag, ref contexts) =>
                match self.write_head(types::RDISPATCH, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_contexts(contexts.as_slice())
                },

            &Tdrain(tag) => self.write_head(types::TDRAIN, tag),
            &Rdrain(tag) => self.write_head(types::RDRAIN, tag),

            &Tping(tag) => self.write_head(types::TPING, tag),
            &Rping(tag) => self.write_head(types::RPING, tag),

            &Tdiscarded(which, ref msg) =>
                match self.write_head(types::TDISCARDED, MARKER_TAG) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_tag(which) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write_str(msg.as_slice())
                    }
                },

            &Tlease(unit, amount) =>
                match self.write_head(types::TLEASE, MARKER_TAG) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_u8(unit) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write_be_u64(amount)
                    }
                },

            &Rerr(tag, ref msg) =>
                match self.write_head(types::RERR, tag) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(msg.as_slice())
                },
        }
    }

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
        let bytes = s.as_bytes();
        match self.write_be_u16(bytes.len() as u16) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write(bytes)
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

    fn write_tag(&mut self, tag: Tag) -> IoResult<()> {
        let Tag(b0, b1, b2) = tag;
        self.write([b0, b1, b2])
    }

    fn write_head(&mut self, typ: i8, tag: Tag) -> IoResult<()> {
        match self.write_i8(typ) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_tag(tag)
        }
    }
}

impl<W: Writer> MessageWriter for W {}

#[cfg(test)]
mod test {
    use std::io::MemWriter;
    use proto::{Message, Tdiscarded, Tag};
    use super::MessageWriter;

    fn encode_message(m: &Message) -> Vec<u8> {
        let mut w = MemWriter::new();
        w.write_message(m).ok();
        w.unwrap()
    }

    #[test]
    fn test_discarded() {
        let vec = encode_message(&Tdiscarded(Tag(0, 1, 2), "BAD".to_string()));
        assert_eq!(vec, vec![
            66, // type
            0, 0, 0, // marker tag
            0, 1, 2, // tag ref
            66, 65, 68, // msg: BAD
            ])
    }
}
