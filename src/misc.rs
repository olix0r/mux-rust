
#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Path(pub Vec<Vec<u8>>);
impl Path {
    #[inline]
    pub fn empty() -> Path { Path(Vec::with_capacity(0)) }

    // TODO
    pub fn from_str(_: String) -> Path { Path::empty() }
    pub fn from_bytes(_: Vec<u8>) -> Path { Path::empty() }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dentry { pub src: Path, pub tgt: Vec<u8> }

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Dtab(pub Vec<Dentry>);
impl Dtab {
    #[inline]
    pub fn empty() -> Dtab { Dtab(Vec::with_capacity(0)) }
}

#[deriving(Clone,PartialEq,Eq,Show)]
pub struct Context { pub key: Vec<u8>, pub val: Vec<u8> }
