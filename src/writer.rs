#[allow(unstable)]

use std::io::{IoResult, Writer, MemWriter};

use misc::{Context, Dtab, Dentry, Trace};
use proto::{Tag, Msg};

pub trait FrameWriter: Writer {
    fn write_be_u32_frame(&mut self, frame: &[u8]) -> IoResult<()> {
        self.write_be_u32(frame.len() as u32)
            .and_then(|_| self.write(frame))
    }
}

impl<W: Writer> FrameWriter for W {}

pub trait MuxWriter: FrameWriter {

    fn write_mux_frame<'t>(&mut self, tag: &'t Tag, msg: &'t Msg) -> IoResult<()> {
        let mut buf = MemWriter::new();
        buf.write_mux(tag, msg).and_then(move|_| {
            self.write_be_u32_frame(buf.into_inner().as_slice())
        })
    }

    fn write_mux<'t>(&mut self, tag: &'t Tag, msg: &'t Msg) -> IoResult<()> {
        self.write_i8(msg.get_type().to_i8())
            .and_then(|_| self.write_mux_tag(tag))
            .and_then(|_| self.write_mux_msg(msg))
    }

    fn write_mux_tag<'t>(&mut self, tag: &'t Tag) -> IoResult<()> {
        let &Tag(b0, b1, b2) = tag;
        self.write(&[b0, b1, b2])
    }

    fn write_mux_msg(&mut self, m: &Msg) -> IoResult<()> {
        match m {
            &Msg::Treq(ref trace, ref body) => {
                self.write_mux_trace(trace).and_then(|_| self.write(body.as_slice()))
            },

            &Msg::RreqOk(ref body) => self.write(body.as_slice()),
            &Msg::RreqError(ref s) => self.write_str(s.as_slice()),
            &Msg::RreqNack => Ok(()),

            &Msg::Tdispatch(ref contexts, ref dst, ref dtab, ref body) => {
                self.write_mux_contexts(contexts.as_slice())
                    .and_then(|_| self.write_len_str(dst.as_slice()))
                    .and_then(|_| self.write_mux_dtab(dtab))
                    .and_then(|_| self.write(body.as_slice()))
            },

            &Msg::RdispatchOk(ref contexts, ref body) => {
                self.write_u8(0) // status
                    .and_then(|_| self.write_mux_contexts(contexts.as_slice()))
                    .and_then(|_| self.write(body.as_slice()))
            },
            &Msg::RdispatchError(ref contexts, ref msg) => {
                self.write_u8(1) // status
                    .and_then(|_| self.write_mux_contexts(contexts.as_slice()))
                    .and_then(|_| self.write_str(msg.as_slice()))
            },
            &Msg::RdispatchNack(ref contexts) => {
                self.write_u8(2).and_then(|_| {
                    self.write_mux_contexts(contexts.as_slice())
                })
            },

            &Msg::Tdrain | &Msg::Rdrain | &Msg::Tping | &Msg::Rping => {
                Ok(())
            },

            &Msg::Tdiscarded(ref which, ref msg) => {
                self.write_mux_tag(which).and_then(|_| self.write_str(msg.as_slice()))
            },

            &Msg::Tlease(unit, amount) => {
                self.write_u8(unit).and_then(|_| self.write_be_u64(amount))
            },

            &Msg::Rerr(ref msg) => {
                self.write_str(msg.as_slice())
            },
        }
    }

    fn write_len_vec<T, F: FnMut(&mut Self, &T) -> IoResult<()>>(
        &mut self,
        ts: &[T],
        mut f: F
    ) -> IoResult<()> {
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
        self.write_be_u16(buf.len() as u16)
            .and_then(|_| self.write(buf))
    }

    fn write_len_str(&mut self, s: &str) -> IoResult<()> {
        let bytes = s.as_bytes();
        self.write_be_u16(bytes.len() as u16)
            .and_then(|_| self.write(bytes))
    }

    fn write_mux_context(&mut self, context: &Context) -> IoResult<()> {
        self.write_len_buf(context.key.as_slice())
            .and_then(|_| self.write_len_buf(context.val.as_slice()))
    }

    fn write_mux_contexts(&mut self, contexts: &[Context]) -> IoResult<()> {
        self.write_len_vec(contexts, |w, ctx| w.write_mux_context(ctx))
    }

    fn write_mux_dentry(&mut self, dentry: &Dentry) -> IoResult<()> {
        self.write_len_str(dentry.src.as_slice())
            .and_then(|_| self.write_len_str(dentry.tree.as_slice()))
    }

    fn write_mux_dtab(&mut self, dtab: &Dtab) -> IoResult<()> {
        let &Dtab(ref dentries) = dtab;
        self.write_len_vec(dentries.as_slice(), |w, d| w.write_mux_dentry(d))
    }

    fn write_mux_trace<'t>(&mut self, trace: &'t Option<Trace>) -> IoResult<()> {
        match *trace {
            None => self.write_u8(0),

            Some(ref trace) => {
                // two trace variables:
                self.write_u8(2)
                    .and_then(|_| self.write_u8(0)) // var 0: trace id
                    .and_then(|_| self.write_u8(24)) // 3 u64 ids:
                    .and_then(|_| self.write_be_u64(trace.span_id))
                    .and_then(|_| self.write_be_u64(trace.parent_id))
                    .and_then(|_| self.write_be_u64(trace.trace_id))
                    .and_then(|_| self.write(&[1, 1, trace.flags])) // var 1: flags
            }
        }
    }

}

impl<W: FrameWriter> MuxWriter for W {}

#[cfg(test)]
mod test {
    use std::io::MemWriter;
    use proto::{Msg, Tag};
    use super::MuxWriter;

    fn encode_frame(tag: Tag, msg: Msg) -> Vec<u8> {
        let mut buf = MemWriter::new();
        buf.write_mux_frame(&tag, &msg).ok();
        buf.into_inner()
    }

    #[test]
    fn test_discarded() {
        let vec = encode_frame(Tag(0, 0, 0), Msg::Tdiscarded(Tag(0, 1, 2), "BAD".to_string()));
        assert_eq!(vec, vec![
            00, 00, 00, 10, // size
            66, // type
            0, 0, 0, // marker tag
            0, 1, 2, // tag ref
            66, 65, 68, // msg: BAD
            ]);
    }
}
