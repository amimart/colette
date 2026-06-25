use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind, StoreKey};
use crate::iter::IndexIterator;
use crate::key::Key;
use crate::prefix::{Prefix, Prefixable};
use crate::store::{MultiStoreReadHandle, ReadKVStore};
use std::marker::PhantomData;
use std::ops::{Bound, Range, RangeBounds};

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
            PrefixOrKey::Key(k) => k.encode().as_ref().to_vec(),
        }
    }
}

pub enum ScanRange {
    All,
    Prefix {
        start: Bound<Vec<u8>>,
        end: Bound<Vec<u8>>,
    },
    Range {
        left: Bound<Vec<u8>>,
        right: Bound<Vec<u8>>,
    },
}

impl ScanRange {
    pub fn prefix(prefix: Vec<u8>) -> Self {
        if prefix.is_empty() {
            return ScanRange::All;
        }

        let end = Self::prefix_end(&prefix);

        Self::Prefix {
            start: Bound::Included(prefix),
            end,
        }
    }

    fn prefix_end(bytes: &[u8]) -> Bound<Vec<u8>> {
        let mut out = bytes.to_vec();
        for i in (0..out.len()).rev() {
            if out[i] != 0xff {
                out[i] += 1;
                out.truncate(i + 1);
                return Bound::Excluded(out);
            }
        }

        Bound::Unbounded
    }
}

impl RangeBounds<Vec<u8>> for ScanRange {
    fn start_bound(&self) -> Bound<&Vec<u8>> {
        match self {
            ScanRange::All => Bound::Unbounded,
            ScanRange::Prefix { start, end: _ } => start.as_ref(),
            ScanRange::Range { left, right: _ } => left.as_ref(),
        }
    }

    fn end_bound(&self) -> Bound<&Vec<u8>> {
        match self {
            ScanRange::All => Bound::Unbounded,
            ScanRange::Prefix { start: _, end } => end.as_ref(),
            ScanRange::Range { left: _, right } => right.as_ref(),
        }
    }
}

pub(crate) fn prefix_bounds(prefix: Vec<u8>) -> (Bound<Vec<u8>>, Bound<Vec<u8>>) {
    if prefix.is_empty() {
        return (Bound::Unbounded, Bound::Unbounded);
    }

    let right = ScanRange::prefix_end(&prefix);
    (Bound::Included(prefix), right)
}

pub struct IndexScan<'a, ReadHandle, Record, Idx>
where
    Self: 'a,
    ReadHandle: MultiStoreReadHandle,
    Record: Entity,
    Idx: Index<Record>,
    for<'b> Idx::Kind<'b>: IndexKind<Idx::Key<'b>, Record::Key<'b>>,
{
    collection_name: &'static str,
    read_handle: ReadHandle,
    left: Bound<Vec<u8>>,
    right: Bound<Vec<u8>>,
    direction: Direction,
    after: Option<StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>>,

    _marker: PhantomData<(Record, Idx)>,
}

impl<'a, ReadHandle, Record, Idx> IndexScan<'a, ReadHandle, Record, Idx>
where
    ReadHandle: MultiStoreReadHandle,
    Record: Entity,
    Idx: Index<Record>,
    for<'b> Idx::Kind<'b>: IndexKind<Idx::Key<'b>, Record::Key<'b>>,
{
    pub fn new(collection_name: &'static str, read_handle: ReadHandle) -> Self {
        Self {
            collection_name,
            read_handle,
            left: Bound::Unbounded,
            right: Bound::Unbounded,
            direction: Direction::LeftToRight,
            after: None,

            _marker: PhantomData,
        }
    }

    pub fn range(
        mut self,
        range: Range<Bound<StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>>>,
    ) -> Self {
        self.left = range.start.map(|p| p.encode().as_ref().to_vec());
        self.right = range.end.map(|p| p.encode().as_ref().to_vec());
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn after(mut self, cursor: StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>) -> Self {
        self.after = Some(cursor);
        self
    }

    pub fn iter(self) -> Result<IndexIterator<ReadHandle::Store, Record>, Error> {
        Ok(IndexIterator::new(
            self.read_handle
                .open_store(Idx::NAME)?
                .scan((self.left, self.right), self.direction)?,
            self.read_handle.open_store(self.collection_name)?,
        ))
    }
}

pub trait PrefixScan<StoredKey: Key + Prefixable<KeyPrefix>, KeyPrefix: Prefix> {
    fn prefix(self, prefix: KeyPrefix) -> Self;

    fn prefix_range(self, range: Range<Bound<KeyPrefix>>) -> Self;

    fn range(self, range: Range<Bound<PrefixOrKey<StoredKey, KeyPrefix>>>) -> Self;
}

impl<'a, ReadHandle, Record, Idx, KeyPrefix>
    PrefixScan<StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>, KeyPrefix>
    for IndexScan<'a, ReadHandle, Record, Idx>
where
    ReadHandle: MultiStoreReadHandle,
    Record: Entity,
    Idx: Index<Record>,
    KeyPrefix: Prefix,
    StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>: Key + Prefixable<KeyPrefix>,
    for<'b> Idx::Kind<'b>: IndexKind<Idx::Key<'b>, Record::Key<'b>>,
{
    fn prefix(mut self, prefix: KeyPrefix) -> Self {
        (self.left, self.right) = prefix_bounds(prefix.encode_prefix());
        self
    }

    fn prefix_range(mut self, range: Range<Bound<KeyPrefix>>) -> Self {
        self.left = range.start.map(|p| p.encode_prefix());
        self.right = range.end.map(|p| p.encode_prefix());
        self
    }

    fn range(
        mut self,
        range: Range<Bound<PrefixOrKey<StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>, KeyPrefix>>>,
    ) -> Self {
        self.left = range.start.map(|p| p.encode());
        self.right = range.end.map(|p| p.encode());
        self
    }
}
