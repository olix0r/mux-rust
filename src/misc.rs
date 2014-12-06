#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dentry { pub src: String, pub tree: String }

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dtab(pub Vec<Dentry>);
impl Dtab {
    #[inline]
    pub fn empty() -> Dtab { Dtab(Vec::with_capacity(0)) }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Context { pub key: Vec<u8>, pub val: Vec<u8> }

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Contexts(pub Vec<Context>);

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Trace {
    pub span_id: u64,
    pub parent_id: u64,
    pub trace_id: u64,
    pub flags: u64,
}
