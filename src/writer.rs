use std::io::{IoResult, Writer, MemWriter};

use misc::{Context, Dtab, Dentry, Trace};
use proto::{Header, Tag, MARKER_TAG, Message,
            Treq, RreqOk, RreqError, RreqNack,
            Tdispatch, RdispatchOk, RdispatchError, RdispatchNack,
            Tdrain, Rdrain,
            Tping, Rping,
            Tdiscarded,
            Tlease,
            Rerr};

pub trait MessageWriter : Writer {

    fn write_message_body(&mut self, m: &Message) -> IoResult<()> {
        match m {
            &Treq(trace, ref body) =>
                match self.write_trace(trace) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                },

            &RreqOk(ref body) => self.write(body.as_slice()),
            &RreqError(ref s) => self.write_str(s.as_slice()),
            &RreqNack => Ok(()),

            &Tdispatch(ref contexts, ref dst, ref dtab, ref body) =>
                match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_len_str(dst.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => match self.write_dtab(dtab) {
                            Err(ioe) => Err(ioe),
                            Ok(_) => self.write(body.as_slice())
                        }
                    }
                },

            &RdispatchOk(ref contexts, ref body) =>
                match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                },
            &RdispatchError(ref contexts, ref msg) =>
                match self.write_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(msg.as_slice())
                },
            &RdispatchNack(ref contexts) =>
                self.write_contexts(contexts.as_slice()),

            &Tdrain | &Rdrain | &Tping | &Rping => Ok(()),

            &Tdiscarded(which, ref msg) =>
                match self.write_tag(which) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_str(msg.as_slice())
                },

            &Tlease(unit, amount) =>
                match self.write_u8(unit) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_be_u64(amount)
                },

            &Rerr(ref msg) =>
                self.write_str(msg.as_slice()),
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

    fn write_header(&mut self, hdr: Header) -> IoResult<()> {
        let Header(len, typ, tag) = hdr;
        match self.write_be_u32(len) {
            Err(ioe) => Err(ioe),
            Ok(_) => match self.write_i8(typ as i8) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write_tag(tag)
            }
        }
    }

    fn write_framed(&mut self, tag: Tag, msg: &Message) -> IoResult<()> {
        let mut buf = MemWriter::new();
        match buf.write_message_body(msg) {
            Err(ioe) => Err(ioe),
            Ok(_) => {
                let vec = buf.unwrap();
                let bytes = vec.as_slice();
                match self.write_header(Header(bytes.len() as u32, msg.get_type(), tag)) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(bytes)
                }
            }
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
        let vec = encode_framed(Tag(0, 1, 2), &Tdiscarded("BAD".to_string()));
        assert_eq!(vec, vec![
            00, 00, 00, 10, // size
            66, // type
            0, 0, 0, // marker tag
            0, 1, 2, // tag ref
            66, 65, 68, // msg: BAD
            ])
    }
}
