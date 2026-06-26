use std::collections::Bound;
use crate::key::Key;
use crate::prefix::{Prefix, PrefixOrKey, Prefixable};

pub(crate) type ScanBound = Bound<Vec<u8>>;
pub(crate) type ScanRange = (ScanBound, ScanBound);

pub(crate) trait IntoScanBounds {
    fn start_bound(&self) -> ScanBound;

    fn end_bound(&self) -> ScanBound;
}

impl<P: Prefix> IntoScanBounds for Bound<P> {
    fn start_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(prefix) => prefix.start_bound(),
            Bound::Excluded(prefix) => prefix.end_bound(),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(prefix) => prefix.end_bound(),
            Bound::Excluded(prefix) => match prefix.start_bound() {
                Bound::Included(bytes) | Bound::Excluded(bytes) => Bound::Excluded(bytes),
                Bound::Unbounded => Bound::Unbounded,
            },
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

impl<K: Key + Prefixable<P>, P: Prefix> IntoScanBounds for Bound<PrefixOrKey<K, P>> {
    fn start_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(PrefixOrKey::Prefix(prefix)) => prefix.start_bound(),
            Bound::Excluded(PrefixOrKey::Prefix(prefix)) => prefix.end_bound(),
            Bound::Included(PrefixOrKey::Key(key)) => Bound::Included(key.encode().as_ref().to_vec()),
            Bound::Excluded(PrefixOrKey::Key(key)) => Bound::Excluded(key.encode().as_ref().to_vec()),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(PrefixOrKey::Prefix(prefix)) => prefix.end_bound(),
            Bound::Excluded(PrefixOrKey::Prefix(prefix)) => match prefix.start_bound() {
                Bound::Included(bytes) | Bound::Excluded(bytes) => Bound::Excluded(bytes),
                Bound::Unbounded => Bound::Unbounded,
            },
            Bound::Included(PrefixOrKey::Key(key)) => Bound::Included(key.encode().as_ref().to_vec()),
            Bound::Excluded(PrefixOrKey::Key(key)) => Bound::Excluded(key.encode().as_ref().to_vec()),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}
