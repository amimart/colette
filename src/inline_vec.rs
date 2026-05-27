use smallvec::SmallVec;

pub const MAX_INLINE_VEC_SIZE: usize = 128;
pub type IVec = SmallVec<[u8; MAX_INLINE_VEC_SIZE]>;
