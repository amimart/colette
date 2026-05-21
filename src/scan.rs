use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind, StoreKey};
use crate::iter::IndexIterator;
use crate::key::Key;
use crate::prefix::{Prefix, Prefixable};
use crate::store::{MultiStoreReadHandle, ReadKVStore};
use std::marker::PhantomData;
use std::ops::{Bound, Range};

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
    Range {
        left: Bound<Vec<u8>>,
        right: Bound<Vec<u8>>,
    },
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
    collection_name: &'static str,
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
    pub fn new(collection_name: &'static str, read_handle: ReadHandle) -> Self {
        Self {
            collection_name,
            read_handle,
            range: ScanRange::All,
            direction: Direction::LeftToRight,
            after: None,

            _marker: PhantomData,
        }
    }

    pub fn range(mut self, range: Range<Bound<StoreKey<'a, Idx, PrimaryKey, Record>>>) -> Self {
        self.range = ScanRange::Range {
            left: range.start.map(|p| p.encode()),
            right: range.end.map(|p| p.encode()),
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

    pub fn iter(self) -> Result<IndexIterator<ReadHandle::Store, PrimaryKey, Record>, Error> {
        Ok(IndexIterator::new(
            self.read_handle
                .open_store(Idx::NAME)?
                .scan(self.range, self.direction)?,
            self.read_handle.open_store(&self.collection_name)?,
        ))
    }
}

pub trait PrefixScan<StoredKey: Key + Prefixable<KeyPrefix>, KeyPrefix: Prefix> {
    fn prefix(self, prefix: KeyPrefix) -> Self;

    fn prefix_range(self, range: Range<Bound<KeyPrefix>>) -> Self;

    fn range(self, range: Range<Bound<PrefixOrKey<StoredKey, KeyPrefix>>>) -> Self;
}

impl<'a, ReadHandle, PrimaryKey, Record, Idx, KeyPrefix>
    PrefixScan<StoreKey<'a, Idx, PrimaryKey, Record>, KeyPrefix>
    for IndexScan<'a, ReadHandle, PrimaryKey, Record, Idx>
where
    ReadHandle: MultiStoreReadHandle,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Idx: Index<PrimaryKey, Record>,
    Idx::Kind: IndexKind<Idx::Key, PrimaryKey>,
    KeyPrefix: Prefix,
    StoreKey<'a, Idx, PrimaryKey, Record>: Key + Prefixable<KeyPrefix>,
{
    fn prefix(mut self, prefix: KeyPrefix) -> Self {
        self.range = ScanRange::Prefix(prefix.encode_prefix());
        self
    }

    fn prefix_range(mut self, range: Range<Bound<KeyPrefix>>) -> Self {
        self.range = ScanRange::Range {
            left: range.start.map(|p| p.encode_prefix()),
            right: range.end.map(|p| p.encode_prefix()),
        };
        self
    }

    fn range(
        mut self,
        range: Range<Bound<PrefixOrKey<StoreKey<'a, Idx, PrimaryKey, Record>, KeyPrefix>>>,
    ) -> Self {
        self.range = ScanRange::Range {
            left: range.start.map(|p| p.encode()),
            right: range.end.map(|p| p.encode()),
        };
        self
    }
}
