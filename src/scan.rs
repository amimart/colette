use std::marker::PhantomData;
use std::ops::{Bound, Range};
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

impl<'a, ReadHandle, PrimaryKey, Record, Idx> IndexScan<'a, ReadHandle, PrimaryKey, Record, Idx>
where
    ReadHandle: MultiStoreReadHandle,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Idx: Index<PrimaryKey, Record>,
    Idx::Kind: IndexKind<Idx::Key, PrimaryKey>,
{
    pub fn new(read_handle: ReadHandle) -> Self {
        Self {
            read_handle,
            range: ScanRange::All,
            direction: Direction::LeftToRight,
            after: None,

            _marker: PhantomData,
        }
    }

    pub fn range(mut self, range: Range<Bound<StoreKey<'a, Idx, PrimaryKey, Record>>>) -> Self {
        self.range = ScanRange::Range {
            left: range.start.map(|p |p.encode()),
            right: range.end.map(|p |p.encode()),
        };
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn after(mut self, cursor: StoreKey<'a, Idx, PrimaryKey, Record>) -> Self {
        self.after = Some(cursor);
        self
    }
}
