use std::io::{IoResult, Writer, MemWriter};

use misc::{Context, Dtab, Dentry, Trace, Detailed};
use proto::{Tag, Message,
            Treq, RreqOk, RreqError, RreqNack,
            Tdispatch, RdispatchOk, RdispatchError, RdispatchNack,
            Tdrain, Rdrain,
            Tping, Rping,
            Tdiscarded,
            Tlease,
            Rerr};

pub trait FrameWriter: Writer {
    fn write_be_u32_frame(&mut self, frame: &[u8]) -> IoResult<()> {
        match self.write_be_u32(frame.len() as u32) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write(frame)
        }
    }
}

impl<W: Writer> FrameWriter for W {}

pub trait MuxWriter: FrameWriter {

    fn write_mux_frame(&mut self, tag: Tag, msg: Message) -> IoResult<()> {
        let mut buf = MemWriter::new();
        match buf.write_mux(tag, msg) {
            Err(ioe) => Err(ioe),

            Ok(_) => {
                let bytes = buf.unwrap();
                println!("write frame: {}",
                         bytes.iter().fold(String::new(), |s,&b| -> String {
                             format!("{}{:02x}", s, b)
                         }));
                self.write_be_u32_frame(bytes.as_slice())
            }
        }
    }

    fn write_mux(&mut self, tag: Tag, msg: Message) -> IoResult<()> {
         match self.write_i8(msg.get_type() as i8) {
            Err(ioe) => Err(ioe),
            Ok(_) => match self.write_mux_tag(tag) {
                Err(ioe) => Err(ioe),
                Ok(_) => self.write_mux_message(&msg)
            }
        }
    }

    fn write_mux_tag(&mut self, tag: Tag) -> IoResult<()> {
        let Tag(b0, b1, b2) = tag;
        self.write([b0, b1, b2])
    }

    fn write_mux_message(&mut self, m: &Message) -> IoResult<()> {
        println!("writing {}", m.get_type());
        match m {
            &Treq(trace, ref body) =>
                match self.write_mux_trace(trace) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write(body.as_slice())
                },

            &RreqOk(ref body) => self.write(body.as_slice()),
            &RreqError(ref s) => self.write_str(s.as_slice()),
            &RreqNack => Ok(()),

            &Tdispatch(ref contexts, ref dst, ref dtab, ref body) =>
                match self.write_mux_contexts(contexts.as_slice()) {
                    Err(ioe) => Err(ioe),

                    Ok(_) => match self.write_len_str(dst.as_slice()) {
                        Err(ioe) => Err(ioe),

                        Ok(_) => match self.write_mux_dtab(dtab) {
                            Err(ioe) => Err(ioe),

                            Ok(_) => self.write(body.as_slice())
                        }
                    }
                },

            &RdispatchOk(ref contexts, ref body) =>
                match self.write_u8(0) { // status
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_mux_contexts(contexts.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write(body.as_slice())
                    },
                },
            &RdispatchError(ref contexts, ref msg) =>
                match self.write_u8(1) { // status
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_mux_contexts(contexts.as_slice()) {
                        Err(ioe) => Err(ioe),
                        Ok(_) => self.write_str(msg.as_slice())
                    }
                },
            &RdispatchNack(ref contexts) =>
                match self.write_u8(2) { // status
                    Err(ioe) => Err(ioe),
                    Ok(_) => self.write_mux_contexts(contexts.as_slice())
                },

            &Tdrain | &Rdrain | &Tping | &Rping => Ok(()),

            &Tdiscarded(which, ref msg) =>
                match self.write_mux_tag(which) {
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

    fn write_mux_context(&mut self, context: &Context) -> IoResult<()> {
        println!("writing context {}", context);
        match self.write_len_buf(context.key.as_slice()) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_len_buf(context.val.as_slice())
        }
    }

    fn write_mux_contexts(&mut self, contexts: &[Context]) -> IoResult<()> {
        println!("writing {} contexts", contexts.len());
        self.write_len_vec(contexts, |w, ctx| -> IoResult<()> {
            w.write_mux_context(ctx)
        })
    }

    fn write_mux_dentry(&mut self, dentry: &Dentry) -> IoResult<()> {
        match self.write_len_str(dentry.src.as_slice()) {
            Err(ioe) => Err(ioe),
            Ok(_) => self.write_len_str(dentry.tree.as_slice())
        }
    }

    fn write_mux_dtab(&mut self, dtab: &Dtab) -> IoResult<()> {
        let &Dtab(ref dentries) = dtab;
        self.write_len_vec(dentries.as_slice(), |w, d| -> IoResult<()> { w.write_mux_dentry(d) })
    }

    fn write_mux_trace(&mut self, trace: Option<Trace>) -> IoResult<()> {
        match trace {
            None => self.write_u8(0),

            Some(trace) => match self.write_u8(2) { // two trace variables:
                Err(ioe) => Err(ioe),

                // var 0: trace id
                Ok(_) => match self.write_u8(0) {
                    Err(ioe) => Err(ioe),
                    Ok(_) => match self.write_u8(24) { // 3 u64 ids:
                        Err(ioe) => Err(ioe),
                        Ok(_) => match self.write_be_u64(trace.span_id) {
                            Err(ioe) => Err(ioe),
                            Ok(_) => match self.write_be_u64(trace.parent_id) {
                                Err(ioe) => Err(ioe),
                                Ok(_) => match self.write_be_u64(trace.trace_id) {
                                    Err(ioe) => Err(ioe),

                                    // var 1: flags
                                    Ok(_) => self.write([1, 1, trace.flags])
                                }
                            }
                        }
                    }
                },
            },
        }
    }

}

impl<W: FrameWriter> MuxWriter for W {}

#[cfg(test)]
mod test {
    use std::io::MemWriter;
    use proto::{Message, Tdiscarded, Tag};
    use super::MuxWriter;

    fn encode_frame(tag: Tag, msg: Message) -> Vec<u8> {
        let mut buf = MemWriter::new();
        buf.write_mux_frame(tag, msg).ok();
        buf.unwrap()
    }

    #[test]
    fn test_discarded() {
        let vec = encode_frame(Tag(0, 0, 0), Tdiscarded(Tag(0, 1, 2), "BAD".to_string()));
        assert_eq!(vec, vec![
            00, 00, 00, 10, // size
            66, // type
            0, 0, 0, // marker tag
            0, 1, 2, // tag ref
            66, 65, 68, // msg: BAD
            ]);
    }
}
