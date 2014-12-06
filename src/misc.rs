#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dentry { pub src: String, pub tree: String }
impl Dentry {
    pub fn new(s: String, t: String) -> Dentry { Dentry{src: s, tree: t} }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dtab(pub Vec<Dentry>);
impl Dtab {
    #[inline]
    pub fn empty() -> Dtab { Dtab(Vec::with_capacity(0)) }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Context { pub key: Vec<u8>, pub val: Vec<u8> }
impl Context {
    pub fn new(k: Vec<u8>, v: Vec<u8>) -> Context { Context{key: k, val: v} }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Contexts(pub Vec<Context>);

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Trace {
    pub span_id: u64,
    pub parent_id: u64,
    pub trace_id: u64,
    pub flags: u8,
}

