use crate::error::CodecError;
use crate::key::{HasKey, Key};

pub trait Entity<PrimaryKey>: HasKey<PrimaryKey> + Sized
where
    PrimaryKey: Key,
{
    fn to_bytes(&self) -> Result<Vec<u8>, CodecError>;

    fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError>;
}
