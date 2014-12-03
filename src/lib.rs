/// An implementation of mux in Rust.
/// See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala
pub mod mux {
    // TODO these should be moved into other crates...
    pub mod misc {
        use std::fmt;
        use std::str;

        /// A simple wrapper type for an array of bytes.
        #[deriving(Clone,PartialEq,Eq)]
        pub struct Buf<'t>(&'t [u8]);
        static EMPTY_BUF: Buf<'static> = Buf(&[]);
        impl<'t> Buf<'t> {
            pub fn new(bytes: &'t [u8]) -> Buf<'t> { Buf(bytes) }

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

        #[deriving(Clone,PartialEq,Eq,Show)]
        pub struct Path<'t>(pub &'t [Buf<'t>]);
        static EMPTY_PATH: Path<'static> = Path(&[]);
        impl<'t> Path<'t> {
            #[inline]
            pub fn empty() -> Path<'t> { EMPTY_PATH }

            // TODO
            pub fn from_str(_: &'t [str]) -> Path<'t> { EMPTY_PATH }

            // TODO
            pub fn from_bytes<'u>(_: &'u [u8]) -> Path<'u> { EMPTY_PATH }
        }

        #[deriving(Clone,PartialEq,Eq,Show)]
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

        #[deriving(Clone,PartialEq,Eq,Show)]
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

        #[deriving(Clone,PartialEq,Eq,Show)]
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

        #[deriving(Clone,PartialEq,Eq,Show)]
        pub struct Contexts<'t>(pub &'t [Context<'t>]);
        static EMPTY_CONTEXTS: Contexts<'static> = Contexts(&[]);
        impl<'t> Contexts<'t> {
            #[inline]
            pub fn empty() -> Contexts<'t> { EMPTY_CONTEXTS }
        }

        pub trait Reader<'a> {
            fn len(self) -> uint;
            fn read_u8(self) -> Result<Read<'a, Self, u8>, ReadErr<'a, Self>>;
            fn read_i8(self) -> Result<Read<'a, Self, i8>, ReadErr<'a, Self>>;
            fn read_u16_be(self) -> Result<Read<'a, Self, u16>, ReadErr<'a, Self>>;
            fn read_uint_be(self, nbytes: uint) -> Result<Read<'a, Self, u64>, ReadErr<'a, Self>>;
            fn read_slice(self, nbytes: uint) -> Result<Read<'a, Self, &'a [u8]>, ReadErr<'a, Self>>;
        }
        pub enum ReadErr<'a, R: Reader<'a>> {
            ReadUnderflow(R)
        }

        pub struct Read<'a, R: Reader<'a>, T>(pub R, pub T);

        pub trait FlatMap<'a, R: Reader<'a>, T> {
            fn flat_map<U>(self, f: |Read<'a, R, T>| -> Result<Read<'a, R, U>, ReadErr<'a, R>>) -> Result<Read<'a, R, U>, ReadErr<'a, R>>;
        }

        impl<'a, R: Reader<'a>, T> FlatMap<'a, R, T> for Result<Read<'a, R, T>, ReadErr<'a, R>> {
            fn flat_map<U>(self, f: |Read<'a, R, T>| -> Result<Read<'a, R, U>, ReadErr<'a, R>>) -> Result<Read<'a, R, U>, ReadErr<'a, R>> {
                match self {
                    Err(e) => Err(e),
                    Ok(read) => f(read),
                }
            }
        }

        impl<'a> Reader<'a> for Buf<'a> {

            fn len(self) -> uint {
                self.as_bytes().len()
            }

            fn read_u8(self) -> Result<Read<'a, Buf<'a>, u8>, ReadErr<'a, Buf<'a>>> {
                match self {
                    Buf(bytes) if bytes.len() == 0 =>
                        Err(ReadUnderflow(self)),
                    Buf(bytes) =>
                        Ok(Read(Buf(bytes.slice_from(1)), bytes[0])),
                }
            }

            fn read_i8(self) -> Result<Read<'a, Buf<'a>, i8>, ReadErr<'a, Buf<'a>>> {
                self.read_u8().map(|Read(reader, u)| -> Read<'a, Buf<'a>, i8> { Read(reader, u as i8) })
            }

            fn read_slice(self, nbytes: uint) -> Result<Read<'a, Buf<'a>, &'a [u8]>, ReadErr<'a, Buf<'a>>> {
                match self {
                    Buf(bytes) if bytes.len() < nbytes =>
                        Err(ReadUnderflow(self)),
                    Buf(bytes) =>
                        Ok(Read(Buf(bytes.slice_from(nbytes)), bytes.slice_to(nbytes)))
                }
            }

            /// Lifted from std::io::Reader
            fn read_uint_be(self, nbytes: uint) -> Result<Read<'a, Buf<'a>, u64>, ReadErr<'a, Buf<'a>>> {
                if self.len() < nbytes {
                    return Err(ReadUnderflow(self));
                }
                let mut val = 0u64;
                let mut i = nbytes;
                let mut reader: Buf<'a> = self;
                while i > 0 {
                    i -= 1;
                    match reader.read_u8() {
                        Err(e) => return Err(e),
                        Ok(Read(next_reader, n)) => {
                            val += n as u64 << i * 8;
                            reader = next_reader;
                        },
                    }
                }
                Ok(Read(reader, val))
            }

            fn read_u16_be(self) -> Result<Read<'a, Buf<'a>, u16>, ReadErr<'a, Buf<'a>>> {
                self.read_uint_be(2).map(|Read(r, n)| -> Read<'a, Buf<'a>, u16> { Read(r, n as u16) })
            }
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
        use super::misc::{Buf, Context, Contexts, Path, Dtab, Reader, Read, ReadErr, ReadUnderflow, FlatMap};

        #[deriving(Clone,PartialEq,Eq,Show)]
        pub struct Tag(u64);

        #[deriving(Clone,PartialEq,Eq,Show)]
        pub enum Type {
            TreqType,
            RreqType,
            TdispatchType,
            RdispatchType,
            TdrainType,
            RdrainType,
            TpingType,
            RpingType,
            TdiscardedType,
            TleaseType,
            RerrType,
        }

        #[deriving(Clone,PartialEq,Eq,Show)]
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

        #[deriving(Clone,PartialEq,Eq,Show)]
        pub enum DecodeErr<E> {
            DecodeUnderflow,
            UnknownType(i8),
            OtherErr(E),
        }

        pub type Decoded<T, E> = Result<T, DecodeErr<E>>;

        impl<'t> Message<'t> {

            fn decode_type<R: Reader<'t>, T, E>(reader: R, f: |R, Type| -> Decoded<T, E>) -> Decoded<T, E> {
                match reader.read_i8() {
                    Err(ReadUnderflow(_)) => Err(DecodeUnderflow),
                    Ok(Read(reader, val)) => match val {
                         1 => f(reader, TreqType),
                        -1 => f(reader, RreqType),

                         2 => f(reader, TdispatchType),
                        -2 => f(reader, RdispatchType),

                         64 => f(reader, TdrainType),
                        -64 => f(reader, RdrainType),

                         65 => f(reader, TpingType),
                        -65 => f(reader, RpingType),

                        66 => f(reader, TdiscardedType),
                        67 => f(reader, TleaseType),

                        -128 => f(reader, RerrType),

                        t => Err(UnknownType(t))
                    }
                }
            }

            fn decode_tag<R: Reader<'t>, T, E>(reader: R, f: |R, Tag| -> Decoded<T, E>) -> Decoded<T, E> {
                match reader.read_uint_be(3) {
                    Err(ReadUnderflow(_)) => Err(DecodeUnderflow),
                    Ok(Read(r, n)) => f(r, Tag(n)),
                }
            }

            fn decode_buf<R: Reader<'t>, T, E>(reader: R, f: |R, Buf<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                let read_slice = reader.read_u16_be().flat_map(
                    |Read(reader, nbytes)| -> Result<Read<'t, R, &'t [u8]>, ReadErr<'t, R>> {
                        reader.read_slice(nbytes as uint)
                    });
                match read_slice {
                    Err(ReadUnderflow(_)) => Err(DecodeUnderflow),
                    Ok(Read(reader, slice)) => f(reader, Buf::new(slice)),
                }
            }

            fn decode_context<R: Reader<'t>, T, E>(reader: R, f: |R, Context<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                Message::decode_buf(reader, |reader, key| -> Decoded<T, E> {
                    Message::decode_buf(reader, |reader, val| -> Decoded<T, E> {
                        f(reader, Context(key, val))
                    })
                })
            }

            fn decode_contexts<R: Reader<'t>, T, E>(reader: R, f: |R, Contexts<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                match reader.read_u16_be() {
                    Err(ReadUnderflow(_)) => Err(DecodeUnderflow),

                    Ok(Read(reader, _n)) =>
                        if _n == 0 {
                            f(reader, Contexts::empty())
                        } else {
                            let n = _n as uint;
                            let mut vec: Vec<Context> = Vec::with_capacity(n);
                            for i in range(0, n) {
                                vec.insert(i, Context(Buf::empty(), Buf::empty()));
                            }
                            //let slice = vec.as_slice();
                            // FIXME lifetime
                            f(reader, Contexts::empty())
                        }
                }
            }

            fn decode_destination<R: Reader<'t>, T, E>(reader: R, f: |R, Path<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                Message::decode_buf(reader, |reader, buf| -> Decoded<T, E> {
                    // FIXME
                    f(reader, Path::empty())
                })
            }

            fn with_message<R: Reader<'t>, T, E>(reader: R, f: |R, Message<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                Message::decode_type(reader, |reader, typ| -> Decoded<T, E> {
                    Message::decode_tag(reader, |reader, tag| -> Decoded<T, E> {
                        match typ {
                            TdispatchType => Message::decode_contexts(reader, |_, _| -> Decoded<T, E> {
                                Err(UnknownType(2))
                            }),
                            _ => Err(UnknownType(-128)),
                        }
                    })
                })
            }

            /// Decode a slice of bytes as a mux message.
            pub fn decoded<T, E>(bytes: &'t [u8], f: |Message<'t>| -> Decoded<T, E>) -> Decoded<T, E> {
                let buf = Buf::new(bytes);
                Message::with_message(buf, |_, msg| -> Decoded<T, E> { f(msg) })
            }

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
                    sum + 2+kbuf.len() + 2+vbuf.len()
                })
            }
        }

        #[cfg(test)]
        mod test {
            use mux::misc::{Contexts, Path, Dtab, Buf};
            use super::{Tag, Tdispatch, Message, Decoded};

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

                let mut decoded: Option<Message> = None;
                let result: Decoded<bool, &str> = Message::decoded(bytes.as_slice(), |msg| {
                    decoded = Some(msg);
                    Ok(true)
                 });
                assert_eq!(decoded, Some(orig));
                assert_eq!(result, Ok(true));
            }
        }
    }
}

