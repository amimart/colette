use std::marker::PhantomData;
use std::ops::{Bound};
use crate::entity::Entity;
use crate::index::{Index, IndexKind, StoreKey};
use crate::key::Key;
use crate::prefix::{Prefix, Prefixable};
use crate::store::MultiStoreReadHandle;

pub enum Direction {
    LeftToRight,
    RightToLeft,
}

pub enum PrefixOrKey<K: Key + Prefixable<P>, P: Prefix> {
    Prefix(P),
    Key(K),
}

impl<K, P> PrefixOrKey<K, P>
where
    K: Key + Prefixable<P>,
    P: Prefix,
{
    fn encode(&self) -> Vec<u8> {
        match self {
            PrefixOrKey::Prefix(p) => p.encode_prefix(),
            PrefixOrKey::Key(k) => k.encode(),
        }
    }
}

pub enum ScanRange {
    All,
    Prefix(Vec<u8>),
    Range{ left: Bound<Vec<u8>>, right: Bound<Vec<u8>> },
}

pub struct IndexScan<'a, ReadHandle, PrimaryKey, Record, Idx>
where
    ReadHandle: MultiStoreReadHandle,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Idx: Index<PrimaryKey, Record>,
    Idx::Kind: IndexKind<Idx::Key, PrimaryKey>,
    Self: 'a,
{
    read_handle: ReadHandle,
    range: ScanRange,
    direction: Direction,
    after: Option<StoreKey<'a, Idx, PrimaryKey, Record>>,

    _marker: PhantomData<(PrimaryKey, Record, Idx)>,
}
