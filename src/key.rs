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

impl Key for bool {
    const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<u8>());

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.push(if *self { 1 } else { 0 });
    }

    fn encode(&self) -> Vec<u8> {
        vec![if *self { 1 } else { 0 }]
    }
}

pub fn encode_key_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    for &b in bytes {
        match b {
            0x00 => out.extend_from_slice(&[0x00, 0xff]),
            b => out.push(b),
        }
    }
    out.extend_from_slice(&[0x00, 0x00]);
}

impl Key for String {
    const SIZE: KeySize = KeySize::Variable;

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }

    fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl<const S: usize> Key for [u8; S] {
    const SIZE: KeySize = KeySize::Fixed(S);

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }

    fn encode(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }
}

impl Key for Vec<u8> {
    const SIZE: KeySize = KeySize::Variable;

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }

    fn encode(&self) -> Vec<u8> {
        self.to_vec()
    }
}
