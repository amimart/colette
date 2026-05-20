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

#[macro_export]
macro_rules! key_fixed {
    ($type:ty, $encode:expr) => {
        impl Key for $type {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<Self>());

            fn encode_into(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&$encode(*self));
            }

            fn encode(&self) -> Vec<u8> {
                $encode(*self).to_vec()
            }
        }
    };
}

key_fixed!(u8, u8::to_be_bytes);
key_fixed!(u16, u16::to_be_bytes);
key_fixed!(u32, u32::to_be_bytes);
key_fixed!(u64, u64::to_be_bytes);
key_fixed!(u128, u128::to_be_bytes);
key_fixed!(i8, i8::to_be_bytes);
key_fixed!(i16, i16::to_be_bytes);
key_fixed!(i32, i32::to_be_bytes);
key_fixed!(i64, i64::to_be_bytes);
key_fixed!(i128, i128::to_be_bytes);
key_fixed!(f32, f32::to_be_bytes);
key_fixed!(f64, f64::to_be_bytes);
