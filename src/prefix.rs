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

pub fn prefix_end(mut bytes: Vec<u8>) {
    for i in (0..bytes.len()).rev() {
        if bytes[i] != 0xff {
            bytes[i] += 1;
            bytes.truncate(i + 1);
        }
    }

    bytes.push(0x00);
}
