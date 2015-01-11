#[allow(unstable)]

use std::io::{IoResult, IoError, Reader, InvalidInput, BufReader};

use misc::{Context, Dtab, Dentry, Trace, Detailed};
use proto::{Msg, MsgType, Tag};

struct TraceId(u64, u64, u64);

pub trait FrameReader: Reader {
    fn read_be_u32_frame(&mut self) -> IoResult<Vec<u8>> {
        self.read_be_u32().and_then(|sz| {
            self.read_exact(sz as usize)
        })
    }
}

impl<R: Reader> FrameReader for R {}

pub trait MuxReader: FrameReader {

    fn read_mux_frame(&mut self) -> IoResult<(Tag, Msg)> {
        self.read_be_u32_frame().and_then(|bytes| {
            let mut buf = BufReader::new(bytes.as_slice());
            buf.read_mux()
        })
    }

    fn read_mux(&mut self) -> IoResult<(Tag, Msg)> {
        self.read_i8().and_then(move |t| match MsgType::from_i8(t) {
            None => Err(IoError {
                kind: InvalidInput,
                desc: "unknown message type",
                detail: Some(format!("{}", t)),
            }),
            Some(typ) => {
                self.read_mux_tag().and_then(move |tag| {
                    self.read_mux_msg(typ).map(move |msg| (tag, msg))
                })
            }
        })
    }

    fn read_mux_msg(&mut self, msg_type: MsgType) -> IoResult<Msg> {
        match msg_type {
            MsgType::Treq => self.read_mux_treq(),
            MsgType::Rreq => self.read_mux_rreq(),

            MsgType::Tdispatch => self.read_mux_tdispatch(),
            MsgType::Rdispatch => self.read_mux_rdispatch(),

            MsgType::Tdrain => Ok(Msg::Tdrain),
            MsgType::Rdrain => Ok(Msg::Rdrain),

            MsgType::Tping => Ok(Msg::Tping),
            MsgType::Rping => Ok(Msg::Rping),

            MsgType::Tdiscarded => self.read_mux_tdiscarded(),

            MsgType::Tlease => self.read_mux_tlease(),

            MsgType::Rerr => self.read_to_string().map(|msg| Msg::Rerr(msg)),
        }
    }

    fn read_len_vec<T, F: FnMut(&mut Self, usize) -> IoResult<T>>(
        &mut self,
        len: usize,
        mut f: F
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
        self.read_be_u16().and_then(|len| {
            self.read_exact(len as usize)
        })
    }

    fn read_len_string(&mut self) -> IoResult<String> {
        match self.read_len_buf() {
            Err(ioe) => Err(ioe),
            Ok(buf) => String::from_utf8(buf).map_err(|_| IoError {
                kind: InvalidInput,
                desc: "not a utf8 string",
                detail: None,
            })
        }
    }

    fn read_mux_context(&mut self) -> IoResult<Context> {
        self.read_len_buf().and_then(move |key| {
            self.read_len_buf().map(move |val| Context { key: key, val: val })
        })
    }

    fn read_mux_contexts(&mut self) -> IoResult<Vec<Context>> {
        self.read_be_u16().and_then(|len| {
            self.read_len_vec(len as usize, |r, _| r.read_mux_context())
        })
    }

    fn read_mux_dentry(&mut self) -> IoResult<Dentry> {
        self.read_len_string().and_then(move |src| {
            self.read_len_string().map(move |tree| Dentry { src: src, tree: tree })
        })
    }

    fn read_mux_dtab(&mut self) -> IoResult<Dtab> {
        self.read_be_u16().and_then(|len| {
            self.read_len_vec(len as usize, |r, _| r.read_mux_dentry())
                .map(|dentries| Dtab(dentries))
        })
    }

    fn read_mux_tag(&mut self) -> IoResult<Tag> {
        self.read_u8().and_then(|t0| {
            self.read_u8().and_then(|t1| {
                self.read_u8().map(|t2| Tag(t0,t1,t2))
            })
        })
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

                                (2, vsize) => match self.read_exact(vsize as usize) {
                                    Err(ioe) => return Err(ioe.detail("in trace")),

                                    Ok(bytes) => match bytes.last() {
                                        // let the error be handled by
                                        // a subsequent read...
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

                let trace = curr_trace.map(|TraceId(span, parent, trace)| {
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

    fn read_mux_treq(&mut self) -> IoResult<Msg> {
        self.read_mux_trace().and_then(move |trace| {
            self.read_to_end().map(move |bytes| Msg::Treq(trace, bytes))
        })
    }

    fn read_mux_rreq(&mut self) -> IoResult<Msg> {
        self.read_u8().and_then(|status| match status {
            0 => self.read_to_end().map(|buf| Msg::RreqOk(buf)),
            1 => self.read_to_string().map(|msg| Msg::RreqError(msg)),
            2 => Ok(Msg::RreqNack),
            _ => Err(IoError {
                kind: InvalidInput,
                desc: "unknown rreq status",
                detail: None,
            })
        })
    }

    fn read_mux_tdispatch(&mut self) -> IoResult<Msg> {
        self.read_mux_contexts().and_then(move |contexts| {
            self.read_len_string().and_then(move |dst| {
                self.read_mux_dtab().and_then(move |dtab| {
                    self.read_to_end().map(move |body| Msg::Tdispatch(contexts, dst, dtab, body))
                })
            })
        })
    }

    fn read_mux_rdispatch(&mut self) -> IoResult<Msg> {
        self.read_u8().and_then(move |status| {
            self.read_mux_contexts().and_then(move |contexts| {
                match status {
                    0 => self.read_to_end().map(move |body| Msg::RdispatchOk(contexts, body)),
                    1 => self.read_to_string().map(move |desc| Msg::RdispatchError(contexts, desc)),
                    2 => Ok(Msg::RdispatchNack(contexts)),
                    _ => Err(IoError {
                        kind: InvalidInput,
                        desc: "unknown rdispatch status",
                        detail: None,
                    })
                }
            })
        })
    }

    fn read_mux_tdiscarded(&mut self) -> IoResult<Msg> {
        self.read_mux_tag().and_then(|which| {
            self.read_to_string().map(move |msg| Msg::Tdiscarded(which, msg))
        })
    }

    fn read_mux_tlease(&mut self) -> IoResult<Msg> {
        self.read_u8().and_then(|unit| {
            self.read_be_u64().map(|val| Msg::Tlease(unit, val))
        })
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

    fn mk_str_buf(n: usize, s: &str) -> Vec<u8> {
        let mut w = MemWriter::new();
        w.write_be_u16(n as u16).ok();
        w.write_str(s).ok();
        w.into_inner()
    }

    #[test]
    fn test_tag() {
        let bytes = [23, 45, 77, 88];
        let mut r = BufReader::new(&bytes);
        assert_eq!(r.read_mux_tag().unwrap(), Tag(23, 45, 77));
        assert_eq!(r.read_u8().unwrap(), 88);
    }

    #[test]
    fn test_contexts() {
        let bytes = [0x00, 0x00, // contexts
                     0x6e, 0x6f, 0x70, 0x65]; // "nope""
        let mut r = BufReader::new(&bytes);
        assert_eq!(r.read_mux_contexts().unwrap(), vec![]);
        assert_eq!(r.read_u8().unwrap(), 0x6e);
    }

    #[test]
    fn test_len_buf() {
        match mk_reader(&[0, 3, 4, 5, 6, 7]).read_len_buf() {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(buf) => assert_eq!(buf, vec![4, 5, 6])
        }

        match mk_reader(&[0, 3, 4, 5]).read_len_buf() {
            Ok(_) => panic!("did not underflow"),
            Err(_) => (),
        }

        match mk_reader(&[0, 0, 4, 5]).read_len_buf() {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(buf) => assert_eq!(buf, vec![])
        }
    }

    #[test]
    fn test_len_string() {
        match mk_reader(mk_str_buf(3, "mom").as_slice()).read_len_string() {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(s) => assert_eq!(s.as_slice(), "mom")
        }

        match mk_reader(mk_str_buf(3, "mo").as_slice()).read_len_string() {
            Ok(_) => panic!("did not underflow"),
            Err(_) => (),
        }

        match mk_reader(mk_str_buf(0, "mom").as_slice()).read_len_string() {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(s) => assert_eq!(s.as_slice(), "")
        }
    }

    static VEC_BUF: &'static [u8] = &[
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
        0, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 1, 0, 0, 0];

    #[test]
    fn test_len_vec() {
        fn read_u64_vec(n: usize) -> IoResult<Vec<u64>> {
            mk_reader(VEC_BUF).read_len_vec(n, |r, _| r.read_be_u64())
        }

        match read_u64_vec(2) {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(s) => assert_eq!(s, vec![0, 256])
        }

        match read_u64_vec(5) {
            Ok(_) => panic!("did not underflow"),
            Err(_) => (),
        }

        match read_u64_vec(0) {
            Err(ioe) => panic!("read error: {}", ioe),
            Ok(s) => assert_eq!(s, vec![])
        }
    }
}
