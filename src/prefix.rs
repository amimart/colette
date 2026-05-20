use crate::key::Key;

pub trait Prefix {
    fn encode_prefix(&self) -> Vec<u8>;
}

impl<K> Prefix for K
where
    K: Key,
{
    fn encode_prefix(&self) -> Vec<u8> {
        self.encode()
    }
}
