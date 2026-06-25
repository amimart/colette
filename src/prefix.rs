use std::collections::Bound;
use crate::key::Key;

pub trait Prefix {
    fn encode_prefix(&self) -> Vec<u8>;

    fn start_bound(&self) -> Bound<Vec<u8>> {
        let bytes = self.encode_prefix();
        if bytes.is_empty() {
            return Bound::Unbounded;
        }

        Bound::Included(self.encode_prefix())
    }

    fn end_bound(&self) -> Bound<Vec<u8>> {
        let bytes = self.encode_prefix();
        if bytes.is_empty() {
            return Bound::Unbounded;
        }

        prefix_end(bytes)
    }
}

fn prefix_end(mut bytes: Vec<u8>) -> Bound<Vec<u8>> {
    for i in (0..bytes.len()).rev() {
        if bytes[i] != 0xff {
            bytes[i] += 1;
            bytes.truncate(i + 1);
            return Bound::Excluded(bytes);
        }
    }

    Bound::Unbounded
}

impl<K> Prefix for K
where
    K: Key,
{
    fn encode_prefix(&self) -> Vec<u8> {
        self.encode().as_ref().to_vec()
    }
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
