/// An implementation of mux in Rust.
/// See: https://github.com/twitter/finagle/blob/master/finagle-mux/src/main/scala/com/twitter/finagle/mux/package.scala
pub mod mux {

    // TODO
    // It's super lazy and inefficient to be using Vectors here (over, i.e. array slices),
    // but I'm not yet emotionally prepared to manage lifetimes, and ownership and what have you.
    // That'll come next.

    pub struct Path<'t>(&'t [&'t str]);

    pub struct Buf<'t>(&'t [u8]);

    /* TODO

    pub struct Request {
        destination: Path,
        body: Buf,
    }

    pub struct Response {
        body: Buf,
    }
     */

    pub struct Dentry<'t>(&'t str, &'t str);
    pub struct Dtab<'t>(&'t [Dentry<'t>]);

    pub struct Tag(u8, u8, u8);

    pub struct Context<'t>(Buf<'t>, Buf<'t>);
    pub struct Contexts<'t>(&'t [Context<'t>]);

    /*
    pub mod messages {
        pub struct Tdispatch(Tag, Contexts, Path, Dtab, Buf);
        pub struct RdispatchOk(Tag, Contexts, Buf);
        pub struct RdispatchErr(Tag, Contexts, String);
        pub struct RdispatchNack(Tag, Contexts);

        pub struct Tdrain(Tag);
        pub struct Rdrain(Tag);

        pub struct Tping(Tag);
        pub struct Rping(Tag);

        pub struct Tdiscarded(Tag, String);
        pub struct Tlease(u8, u64);

        pub struct Rerr(Tag, String);
    }
     */

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
        Ok(Message<'t>),
        TooShort(uint),
        UnknownType(i8),
    }

    impl<'t> Message<'t> {
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

        pub fn encode(message: Message) -> Vec<u8> {
            match message {
                Tdispatch(tag, Contexts(contexts), path, dtab, buf) => {
                    let csz = (*contexts).iter().fold(0u, |sum, context| -> uint {
                        let Context(Buf(a), Buf(b)) = *context;
                        sum + a.len() + b.len()
                    });
                    Vec::with_capacity(csz)
                },

                _ => Vec::new(),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::{Tag, Contexts, Path, Dtab, Buf, Tdispatch, Message};

        #[test]
        fn encode_tdispatch() {
            let tag = Tag(0,0,1);
            let contexts = Contexts(&[]);

            let p0 = ["foo", "bar", "bah"];
            let path = Path(&p0);

            let dtab = Dtab(&[]);
            let body = Buf(&[]);
            let tdispatch = Tdispatch(tag, contexts, path, dtab, body);
            Message::encode(tdispatch);
        }
    }
}

