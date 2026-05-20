pub trait Key: Clone {
    const SIZE: KeySize;

    fn encode_into(&self, out: &mut Vec<u8>);

    fn encode(&self) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySize {
    Fixed(usize),
    Variable,
}

pub trait HasKey<K: Key> {
    fn key(&self) -> K;
}
