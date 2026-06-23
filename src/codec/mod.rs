use crate::cachestr::Cachestr;
use smallvec::SmallVec;

mod decode;
mod encode;
mod misc;

#[derive(Debug, Default)]
pub struct Context {
    pub(crate) class_refs: SmallVec<[Cachestr; 16]>,
}

pub use decode::*;
pub use encode::*;
