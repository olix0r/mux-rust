/// An implementation of mux in Rust.
/// See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala
pub mod mux {

    // TODO these should be moved into other crates...
    pub mod misc {

        /// A simple wrapper type for an array of bytes.
        pub struct Buf<'t>(&'t [u8]);
        static EMPTY_BUF: Buf<'static> = Buf(&[]);
        impl<'t> Buf<'t> {
            pub fn empty() -> Buf<'t> { EMPTY_BUF }

            pub fn from_string(string: &'t str) -> Buf<'t> { Buf(string.as_bytes()) }

            pub fn as_bytes(&'t self) -> &'t [u8] {
                let Buf(bytes) = *self;
                bytes
            }
        }

        pub struct Path<'t>(pub &'t [Buf<'t>]);
        static EMPTY_PATH: Path<'static> = Path(&[]);
        impl<'t> Path<'t> {
            pub fn empty() -> Path<'t> { EMPTY_PATH }

            // TODO
            pub fn read(str: &'t [str]) -> Path<'t> { EMPTY_PATH }
        }

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

        pub struct Dtab<'t>(pub &'t [Dentry<'t>]);
        static EMPTY_DTAB: Dtab<'static> = Dtab(&[]);
        impl<'t> Dtab<'t> {
            pub fn empty() -> Dtab<'t> { EMPTY_DTAB }

            pub fn dentries(&self) -> &'t [Dentry<'t>] {
                let Dtab(dentries) = *self;
                dentries
            }
        }

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

        pub struct Contexts<'t>(pub &'t [Context<'t>]);
        static EMPTY_CONTEXTS: Contexts<'static> = Contexts(&[]);
        impl<'t> Contexts<'t> {
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
        use super::misc::{Buf, Context, Contexts, Path, Dentry, Dtab};

        pub struct Tag(u8, u8, u8);

        pub enum Message<'t> {
            // Treq(Tag, Option<TraceId>, Buf),
            // RreqOk(Tag, Buf),
            // RreqError(Tag, str),
            // RreqNack(Tag),

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

        pub enum DecodedMessage<'t> {
            Decoded(Message<'t>),
            TooShort(uint),
            UnknownType(i8),
        }

        impl<'t> Message<'t> {

            /// Decode a slice of bytes as a mux message.
            pub fn decode(bytes: &'t [u8]) -> DecodedMessage {
                if bytes.len() < 4 {
                    TooShort(bytes.len())
                } else {
                    let tag_slice = bytes.slice(1, 4);
                    let tag = Tag(tag_slice[0], tag_slice[1], tag_slice[2]);
                    let rest = bytes.slice_from(4);
                    match bytes[0] as i8 {
                        // 1 => Some(decodeTreq(tag, rest)),
                        //-1 => Some(decodeRreq(tag, rest)),
                        kind => UnknownType(kind),
                    }
                }
            }

            /// Encode a mux message as bytes.
            pub fn encode(&self) -> Vec<u8> {
                match *self {
                    Tdispatch(tag, Contexts(contexts), path, dtab, buf) => {
                        let csz = (*contexts).iter().fold(0u, |sum, context| -> uint {
                            let Context(kbuf, vbuf) = *context;
                            sum + kbuf.as_bytes().len() + vbuf.as_bytes().len()
                        });
                        Vec::with_capacity(csz)
                    },

                    _ => Vec::new(),
                }
            }
        }

        #[cfg(test)]
        mod test {
            use super::super::misc::{Contexts, Path, Dtab, Buf};
            use super::{Tag, Tdispatch, Message, Decoded};

            fn mk_tdispatch<'t>() -> Message<'t> {
                let tag = Tag(0,0,1);
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
                    Decoded(Tdispatch(tag, contexts, path, dtab, body)) => (),

                    _ => fail!("could not decode a Tdispatch")
                }
            }
        }
    }
}

