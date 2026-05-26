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

pub trait Prefixable<P>
where
    P: Prefix,
{
}

impl<A, B> Prefixable<A> for (A, B)
where
    A: Key,
    B: Key,
{
}

impl<A, B, C> Prefixable<A> for (A, B, C)
where
    A: Key,
    B: Key,
    C: Key,
{
}

impl<A, B, C> Prefixable<(A, B)> for (A, B, C)
where
    A: Key,
    B: Key,
    C: Key,
{
}

impl<A, B, C, D> Prefixable<A> for (A, B, C, D)
where
    A: Key,
    B: Key,
    C: Key,
    D: Key,
{
}

impl<A, B, C, D> Prefixable<(A, B)> for (A, B, C, D)
where
    A: Key,
    B: Key,
    C: Key,
    D: Key,
{
}

impl<A, B, C, D> Prefixable<(A, B, C)> for (A, B, C, D)
where
    A: Key,
    B: Key,
    C: Key,
    D: Key,
{
}
