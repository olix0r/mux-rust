use std::io::IoError;

#[derive(Clone,PartialEq,Eq,Show)]
pub struct Dentry {
    pub src: String,
    pub tree: String }
impl Dentry {
    #[inline]
    pub fn new(s: String, t: String) -> Dentry { Dentry{src: s, tree: t} }
}

#[derive(Clone,PartialEq,Eq,Show)]
pub struct Dtab(pub Vec<Dentry>);
impl Dtab {
    #[inline]
    pub fn empty() -> Dtab { Dtab(Vec::with_capacity(0)) }
}

#[derive(Clone,PartialEq,Eq,Show)]
pub struct Context { pub key: Vec<u8>, pub val: Vec<u8> }
impl Context {
    pub fn new(k: Vec<u8>, v: Vec<u8>) -> Context { Context{key: k, val: v} }
}

#[derive(Clone,PartialEq,Eq,Show)]
pub struct Contexts(pub Vec<Context>);

#[derive(Clone,Copy,Eq,PartialEq,Show)]
pub struct Trace {
    pub span_id: u64,
    pub parent_id: u64,
    pub trace_id: u64,
    pub flags: u8,
}

pub trait Detailed {
    fn detail(&self, d: &str) -> Self;
}

impl Detailed for IoError {
    fn detail(&self, d: &str) -> IoError {
        IoError {
            kind: self.kind,
            desc: self.desc,
            detail: Some(d.to_string()),
        }
    }
}
