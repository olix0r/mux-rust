/// An implementation of mux in Rust.
/// See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala
pub mod mux {
    // TODO these should be moved into other crates...
    pub mod misc {
        use std::fmt;
        use std::str;

        /// A simple wrapper type for an array of bytes.
        #[deriving(PartialEq,Eq)]
        pub struct Buf<'t>(&'t [u8]);
        static EMPTY_BUF: Buf<'static> = Buf(&[]);
        impl<'t> Buf<'t> {
            #[inline]
            pub fn empty() -> Buf<'t> { EMPTY_BUF }

            pub fn as_bytes(&'t self) -> &'t [u8] {
                let Buf(bytes) = *self;
                bytes
            }

            pub fn as_str(&'t self) -> Option<&'t str> {
                let Buf(bytes) = *self;
                str::from_utf8(bytes)
            }

            pub fn from_str<'u>(s: &'u str) -> Option<Buf<'u>> { Some(Buf(s.as_bytes())) }
        }

        impl<'t> fmt::Show for Buf<'t> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let Buf(bytes) = *self;
                write!(f, "{}", str::from_utf8(bytes))
            }
        }

        #[deriving(PartialEq,Eq,Show)]
        pub struct Path<'t>(pub &'t [Buf<'t>]);
        static EMPTY_PATH: Path<'static> = Path(&[]);
        impl<'t> Path<'t> {
            #[inline]
            pub fn empty() -> Path<'t> { EMPTY_PATH }

            // TODO
            pub fn from_str<'u>(_: &'u [str]) -> Path<'u> { EMPTY_PATH }
        }

        #[deriving(PartialEq,Eq,Show)]
        pub struct Dentry<'t>(pub &'t str, pub &'t str);
        impl<'t> Dentry<'t> {
            pub fn source(&self) -> &'t str {
                let Dentry(source, _) = *self;
                source
            }

            pub fn target(&self) -> &'t str {
                let Dentry(_, target) = *self;
                target
            }
        }

        #[deriving(PartialEq,Eq,Show)]
        pub struct Dtab<'t>(pub &'t [Dentry<'t>]);
        static EMPTY_DTAB: Dtab<'static> = Dtab(&[]);
        impl<'t> Dtab<'t> {
            #[inline]
            pub fn empty() -> Dtab<'t> { EMPTY_DTAB }

            pub fn dentries(&self) -> &'t [Dentry<'t>] {
                let Dtab(dentries) = *self;
                dentries
            }
        }

        #[deriving(PartialEq,Eq,Show)]
        pub struct Context<'t>(pub Buf<'t>, pub Buf<'t>);
        impl<'t> Context<'t> {
            pub fn key(&self) -> Buf<'t> {
                let Context(k, _) = *self;
                k
            }

            pub fn val(&self) -> Buf<'t> {
                let Context(_, v) = *self;
                v
            }
        }

        #[deriving(PartialEq,Eq,Show)]
        pub struct Contexts<'t>(pub &'t [Context<'t>]);
        static EMPTY_CONTEXTS: Contexts<'static> = Contexts(&[]);
        impl<'t> Contexts<'t> {
            #[inline]
            pub fn empty() -> Contexts<'t> { EMPTY_CONTEXTS }
        }
    }

    /*
     * TODO

    pub struct Request {
        destination: Path,
        body: Buf,
    }

    pub struct Response {
        body: Buf,
    }
     */

    /// Mux protocol support for de/serializing mux messages.
    pub mod proto {
        use std::io::{IoResult, IoError, Reader, BufReader};
        use super::misc::{Buf, Context, Contexts, Path, Dtab};

        #[deriving(PartialEq,Eq,Show)]
        pub struct Tag(u64);

        #[deriving(PartialEq,Eq,Show)]
        pub enum Message<'t> {
            // Treq(Tag, Option<TraceId>, Buf),
            // RreqOk(Tag, Buf),
            // RreqError(Tag, str),
            //RreqNack(Tag),

            Tdispatch(Tag, Contexts<'t>, Path<'t>, Dtab<'t>, Buf<'t>),
            RdispatchOk(Tag, Contexts<'t>, Buf<'t>),
            RdispatchError(Tag, Contexts<'t>, &'t str),
            RdispatchNack(Tag, Contexts<'t>),

            Tdrain(Tag),
            Rdrain(Tag),

            Tping(Tag),
            Rping(Tag),
            Tdiscarded(Tag, &'t str),
            Tlease(u8, u64),

            Rerr(Tag, &'t str),
        }

        trait MessageReader<'t> {
            fn read_type(&mut self) -> IoResult<i8>;
            fn read_tag(&mut self) -> IoResult<Tag>;
            fn read_contexts(&mut self) -> IoResult<Contexts<'t>>;

            //fn read_message(&mut self) -> IoResult<Message<'t>, IoError>
        }

        impl<'t, R: Reader> MessageReader<'t> for R {
            fn read_type(&mut self) -> IoResult<i8> { self.read_i8() }

            fn read_tag(&mut self) -> IoResult<Tag> {
                self.read_be_uint_n(3).map(|n| -> Tag { Tag(n) })
            }

            fn read_contexts(&mut self) -> IoResult<Contexts<'t>> {
                match self.read_be_u16() {
                    Err(io) => Err(io),

                    Ok(n) if n == 0 =>
                        Ok(Contexts::empty()),

                    Ok(_n) => {
                        let n = _n as uint;
                        let mut contexts: Vec<Context> = Vec::with_capacity(n);
                        for i in range(0, n) {
                            contexts.insert(i, Context(Buf::empty(), Buf::empty()));
                        }
                        Ok(Contexts(contexts.as_slice().clone()))
                    }
                }
            }
        }

        pub enum DecodeErr {
            IoErr(IoError),
            UnknownType(i8),
        }

        impl<'t> Message<'t> {

            /// Decode a slice of bytes as a mux message.
            pub fn decode<'u>(bytes: &'u [u8]) -> Result<Message<'u>, DecodeErr> {
                let mut reader = BufReader::new(bytes);

                match reader.read_type() {
                    // Ok( 1) => Some(decodeTreq(tag, rest)),
                    // Ok(-1) => Some(decodeRreq(tag, rest)),

                    // Ok( 2) => decodeTdispatch(tag, rest),
                    // Ok(-2) => decodeRdispatch(tag, rest),

                    Ok(kind) => Err(UnknownType(kind)),
                    Err(ioe) => Err(IoErr(ioe)),
                }
            }

            // fn decode_tdispatch<'u>(tag: Tag, bytes: &'u [u8]) -> Decoded<'u, Message<'u>> {
            //     decodeContexts
            // }

            /// Encode a mux message as bytes.
            pub fn encode(&self) -> Vec<u8> {
                match *self {
                    Tdispatch(tag, Contexts(contexts), Path(path), dtab, buf) => {
                        let tagsz = 3;
                        let ctxsz = Message::context_sz(contexts);
                        Vec::with_capacity(tagsz + ctxsz)
                    },

                    _ => Vec::new(),
                }
            }

            fn context_sz<'u>(contexts: &'u [Context]) -> uint {
                (*contexts).iter().fold(0u, |sum, &Context(kbuf, vbuf)| -> uint {
                    sum + kbuf.as_bytes().len() + vbuf.as_bytes().len()
                })
            }
        }

        #[cfg(test)]
        mod test {
            use mux::misc::{Contexts, Path, Dtab, Buf};
            use super::{Tag, Tdispatch, Message, IoErr, UnknownType};

            fn mk_tdispatch<'t>() -> Message<'t> {
                let tag = Tag(1u as u64);
                let contexts = Contexts::empty();
                let path = Path::empty();
                let dtab = Dtab::empty();
                let body = Buf::empty();
                Tdispatch(tag, contexts, path, dtab, body)
            }

            #[test]
            fn tdispatch() {
                let orig = mk_tdispatch();
                let bytes = orig.encode();

                match Message::decode(bytes.as_slice()) {
                    Ok(decoded@Tdispatch(_,_,_,_,_)) =>
                        assert_eq!(orig, decoded),

                    Ok(other) =>
                        fail!("decoded unexpected message type: {}", other),

                    Err(IoErr(ioe)) =>
                        fail!("I/O error: {}", ioe),

                    Err(UnknownType(typ)) =>
                        fail!("unknown message type: {}", typ),
                }
            }
        }
    }
}

