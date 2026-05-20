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

impl<K: Key + ?Sized> Key for &K {
    const SIZE: KeySize = K::SIZE;

    fn encode_into(&self, out: &mut Vec<u8>) {
        (*self).encode_into(out);
    }

    fn encode(&self) -> Vec<u8> {
        (*self).encode()
    }
}

impl<K: Key + ?Sized> Key for (K,) {
    const SIZE: KeySize = K::SIZE;

    fn encode_into(&self, out: &mut Vec<u8>) {
        self.0.encode_into(out);
    }

    fn encode(&self) -> Vec<u8> {
        self.0.encode()
    }
}
