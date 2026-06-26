use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind, StoreKey};
use crate::iter::IndexIterator;
use crate::key::Key;
use crate::prefix::{Prefix, PrefixOrKey, Prefixable};
use crate::store::{MultiStoreReadHandle, ReadKVStore};
use std::marker::PhantomData;
use std::ops::{Bound, Range, RangeBounds};
use crate::bounds::{IntoScanBounds, ScanBound, ScanRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

fn prefix_or_key_left_bound<K, P>(bound: Bound<PrefixOrKey<K, P>>) -> ScanBound
where
    K: Key + Prefixable<P>,
    P: Prefix,
{
    match bound {
        Bound::Included(PrefixOrKey::Prefix(prefix)) => prefix.start_bound(),
        Bound::Excluded(PrefixOrKey::Prefix(prefix)) => prefix.end_bound(),
        Bound::Included(PrefixOrKey::Key(key)) => Bound::Included(key.encode().as_ref().to_vec()),
        Bound::Excluded(PrefixOrKey::Key(key)) => Bound::Excluded(key.encode().as_ref().to_vec()),
        Bound::Unbounded => Bound::Unbounded,
    }
}

fn prefix_or_key_right_bound<K, P>(bound: Bound<PrefixOrKey<K, P>>) -> ScanBound
where
    K: Key + Prefixable<P>,
    P: Prefix,
{
    match bound {
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
    left: ScanBound,
    right: ScanBound,
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
        let (left, right) = match self.after {
            Some(cursor) => Self::apply_cursor(
                self.left,
                self.right,
                self.direction,
                cursor.encode().as_ref().to_vec(),
            )?,
            None => (self.left, self.right),
        };

        Ok(IndexIterator::new(
            self.read_handle
                .open_store(Idx::NAME)?
                .scan((left, right), self.direction)?,
            self.read_handle.open_store(self.collection_name)?,
        ))
    }

    fn apply_cursor(
        left: ScanBound,
        right: ScanBound,
        direction: Direction,
        after: Vec<u8>,
    ) -> Result<ScanRange, Error> {
        if !(left.as_ref(), right.as_ref()).contains(&after) {
            return Err(Error::CursorOutOfBounds);
        }

        Ok(match direction {
            Direction::LeftToRight => (Bound::Excluded(after), right),
            Direction::RightToLeft => (left, Bound::Excluded(after)),
        })
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
        self.left = prefix.start_bound();
        self.right = prefix.end_bound();
        self
    }

    fn prefix_range(mut self, range: Range<Bound<KeyPrefix>>) -> Self {
        self.left = range.start.start_bound();
        self.right = range.end.end_bound();
        self
    }

    fn range(
        mut self,
        range: Range<Bound<PrefixOrKey<StoreKey<'a, 'a, Idx, Record::Key<'a>, Record>, KeyPrefix>>>,
    ) -> Self {
        self.left = prefix_or_key_left_bound(range.start);
        self.right = prefix_or_key_right_bound(range.end);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::error::{CodecError, Error};
    use crate::index::{Index, Multi};
    use crate::key::Key;
    use crate::prefix::encoded_prefix_range;
    use crate::store::MultiStore;
    use crate::testing::{MockDb, ScanLog};

    #[derive(Debug)]
    struct Record {
        id: u32,
        indexed: u32,
    }

    impl Entity for Record {
        type Key<'a> = u32;

        fn key(&self) -> Self::Key<'_> {
            self.id
        }

        fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
            Ok(vec![])
        }

        fn from_bytes(_: &[u8]) -> Result<Self, CodecError> {
            Ok(Self { id: 0, indexed: 0 })
        }
    }

    struct ByNumber;

    impl Index<Record> for ByNumber {
        type Key<'a> = (u32,);
        type Kind<'a> = Multi;
        const NAME: &'static str = "number";

        fn key(entity: &Record) -> Self::Key<'_> {
            (entity.indexed,)
        }
    }

    struct ScanCase {
        name: &'static str,
        setup: ScanSetup,
        direction: Direction,
        after: Option<(u32, u32)>,
        expected: Result<ScanLog, ErrorKind>,
    }

    enum ScanSetup {
        Default,
        Range {
            left: Bound<Vec<u8>>,
            right: Bound<Vec<u8>>,
        },
        Prefix(u32),
        PrefixRange {
            left: Bound<u32>,
            right: Bound<u32>,
        },
        PrefixOrKeyRange {
            left: Bound<RangeEndpoint>,
            right: Bound<RangeEndpoint>,
        },
    }

    enum RangeEndpoint {
        Prefix(u32),
        Key(u32, u32),
    }

    #[derive(Debug, PartialEq, Eq)]
    enum ErrorKind {
        CursorOutOfBounds,
    }

    impl ScanCase {
        fn assert(self) {
            let db = MockDb::new();
            let log = db.log();
            let read = db.read("records").unwrap();

            let scan = IndexScan::<_, Record, ByNumber>::new("records", read);
            let scan = match self.setup {
                ScanSetup::Default => scan,
                ScanSetup::Range { left, right } => {
                    scan.range(left.map(decode_store_key)..right.map(decode_store_key))
                }
                ScanSetup::Prefix(prefix) => scan.prefix(prefix),
                ScanSetup::PrefixRange { left, right } => scan.prefix_range(left..right),
                ScanSetup::PrefixOrKeyRange { left, right } => {
                    PrefixScan::<TestStoreKey, u32>::range(
                        scan,
                        left.map(prefix_or_key)..right.map(prefix_or_key),
                    )
                }
            }
            .direction(self.direction);
            let scan = match self.after {
                Some((index, pk)) => scan.after(store_key(index, pk)),
                None => scan,
            };

            let result = scan.iter();

            match self.expected {
                Ok(expected) => {
                    result.unwrap();
                    assert_eq!(log.borrow().scans, vec![expected], "{}", self.name);
                }
                Err(ErrorKind::CursorOutOfBounds) => {
                    assert!(
                        matches!(result, Err(Error::CursorOutOfBounds)),
                        "{}",
                        self.name
                    );
                    assert!(log.borrow().scans.is_empty(), "{}", self.name);
                }
            }
        }
    }

    #[test]
    fn applies_after_cursor_to_scan_bounds() {
        let cases = vec![
            ScanCase {
                name: "no cursor keeps unbounded scan",
                setup: ScanSetup::Default,
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    Bound::Unbounded,
                    Bound::Unbounded,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "left-to-right cursor tightens left bound",
                setup: range(Bound::Unbounded, Bound::Unbounded),
                direction: Direction::LeftToRight,
                after: Some((2, 20)),
                expected: Ok(scan_log(
                    Bound::Excluded(encode_store_key(2, 20)),
                    Bound::Unbounded,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "right-to-left cursor tightens right bound",
                setup: range(Bound::Unbounded, Bound::Unbounded),
                direction: Direction::RightToLeft,
                after: Some((2, 20)),
                expected: Ok(scan_log(
                    Bound::Unbounded,
                    Bound::Excluded(encode_store_key(2, 20)),
                    Direction::RightToLeft,
                )),
            },
            ScanCase {
                name: "cursor inside included range is valid",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::LeftToRight,
                after: Some((2, 20)),
                expected: Ok(scan_log(
                    Bound::Excluded(encode_store_key(2, 20)),
                    Bound::Included(encode_store_key(3, 30)),
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "reverse cursor inside included range is valid",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::RightToLeft,
                after: Some((2, 20)),
                expected: Ok(scan_log(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Excluded(encode_store_key(2, 20)),
                    Direction::RightToLeft,
                )),
            },
            ScanCase {
                name: "cursor on included left bound is valid",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::LeftToRight,
                after: Some((1, 10)),
                expected: Ok(scan_log(
                    Bound::Excluded(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "cursor on included right bound is valid",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::RightToLeft,
                after: Some((3, 30)),
                expected: Ok(scan_log(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Excluded(encode_store_key(3, 30)),
                    Direction::RightToLeft,
                )),
            },
            ScanCase {
                name: "cursor below left bound fails",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::LeftToRight,
                after: Some((0, 10)),
                expected: Err(ErrorKind::CursorOutOfBounds),
            },
            ScanCase {
                name: "cursor above right bound fails",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::RightToLeft,
                after: Some((4, 10)),
                expected: Err(ErrorKind::CursorOutOfBounds),
            },
            ScanCase {
                name: "cursor on excluded left bound fails",
                setup: range(
                    Bound::Excluded(encode_store_key(1, 10)),
                    Bound::Included(encode_store_key(3, 30)),
                ),
                direction: Direction::LeftToRight,
                after: Some((1, 10)),
                expected: Err(ErrorKind::CursorOutOfBounds),
            },
            ScanCase {
                name: "cursor on excluded right bound fails",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Excluded(encode_store_key(3, 30)),
                ),
                direction: Direction::RightToLeft,
                after: Some((3, 30)),
                expected: Err(ErrorKind::CursorOutOfBounds),
            },
            ScanCase {
                name: "prefix cursor inside bounds is valid",
                setup: ScanSetup::Prefix(2),
                direction: Direction::LeftToRight,
                after: Some((2, 20)),
                expected: Ok(scan_log(
                    Bound::Excluded(encode_store_key(2, 20)),
                    encoded_prefix_range(encode_index_prefix(2)).1,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix cursor outside bounds fails",
                setup: ScanSetup::Prefix(2),
                direction: Direction::LeftToRight,
                after: Some((3, 20)),
                expected: Err(ErrorKind::CursorOutOfBounds),
            },
        ];

        for case in cases {
            case.assert();
        }
    }

    #[test]
    fn configures_iter_scan_bounds_from_public_range_builders() {
        let cases = vec![
            ScanCase {
                name: "default scan is unbounded",
                setup: ScanSetup::Default,
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    Bound::Unbounded,
                    Bound::Unbounded,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "store-key range configures encoded bounds",
                setup: range(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Excluded(encode_store_key(3, 30)),
                ),
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    Bound::Included(encode_store_key(1, 10)),
                    Bound::Excluded(encode_store_key(3, 30)),
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix configures prefix bounds",
                setup: ScanSetup::Prefix(2),
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    encoded_prefix_range(encode_index_prefix(2)).0,
                    encoded_prefix_range(encode_index_prefix(2)).1,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix range excluded upper bound uses encoded prefix",
                setup: ScanSetup::PrefixRange {
                    left: Bound::Included(2),
                    right: Bound::Excluded(4),
                },
                direction: Direction::RightToLeft,
                after: None,
                expected: Ok(scan_log(
                    Bound::Included(encode_index_prefix(2)),
                    Bound::Excluded(encode_index_prefix(4)),
                    Direction::RightToLeft,
                )),
            },
            ScanCase {
                name: "prefix range included upper bound uses prefix end",
                setup: ScanSetup::PrefixRange {
                    left: Bound::Included(2),
                    right: Bound::Included(4),
                },
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    Bound::Included(encode_index_prefix(2)),
                    encoded_prefix_range(encode_index_prefix(4)).1,
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix range excluded lower bound uses prefix end",
                setup: ScanSetup::PrefixRange {
                    left: Bound::Excluded(2),
                    right: Bound::Excluded(4),
                },
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    encoded_prefix_range(encode_index_prefix(2)).1,
                    Bound::Excluded(encode_index_prefix(4)),
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix-or-key range supports prefix lower bound",
                setup: ScanSetup::PrefixOrKeyRange {
                    left: Bound::Included(RangeEndpoint::Prefix(2)),
                    right: Bound::Excluded(RangeEndpoint::Key(3, 30)),
                },
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    Bound::Included(encode_index_prefix(2)),
                    Bound::Excluded(encode_store_key(3, 30)),
                    Direction::LeftToRight,
                )),
            },
            ScanCase {
                name: "prefix-or-key range supports key lower bound",
                setup: ScanSetup::PrefixOrKeyRange {
                    left: Bound::Excluded(RangeEndpoint::Key(2, 20)),
                    right: Bound::Included(RangeEndpoint::Prefix(4)),
                },
                direction: Direction::RightToLeft,
                after: None,
                expected: Ok(scan_log(
                    Bound::Excluded(encode_store_key(2, 20)),
                    encoded_prefix_range(encode_index_prefix(4)).1,
                    Direction::RightToLeft,
                )),
            },
            ScanCase {
                name: "prefix-or-key range excluded prefix lower bound uses prefix end",
                setup: ScanSetup::PrefixOrKeyRange {
                    left: Bound::Excluded(RangeEndpoint::Prefix(2)),
                    right: Bound::Excluded(RangeEndpoint::Prefix(4)),
                },
                direction: Direction::LeftToRight,
                after: None,
                expected: Ok(scan_log(
                    encoded_prefix_range(encode_index_prefix(2)).1,
                    Bound::Excluded(encode_index_prefix(4)),
                    Direction::LeftToRight,
                )),
            },
        ];

        for case in cases {
            case.assert();
        }
    }

    fn scan_log(left: Bound<Vec<u8>>, right: Bound<Vec<u8>>, direction: Direction) -> ScanLog {
        ScanLog {
            left,
            right,
            direction,
        }
    }

    fn range(left: Bound<Vec<u8>>, right: Bound<Vec<u8>>) -> ScanSetup {
        ScanSetup::Range { left, right }
    }

    type TestStoreKey = StoreKey<'static, 'static, ByNumber, u32, Record>;

    fn prefix_or_key(endpoint: RangeEndpoint) -> PrefixOrKey<TestStoreKey, u32> {
        match endpoint {
            RangeEndpoint::Prefix(prefix) => PrefixOrKey::Prefix(prefix),
            RangeEndpoint::Key(index, pk) => PrefixOrKey::Key(store_key(index, pk)),
        }
    }

    fn store_key(index: u32, pk: u32) -> StoreKey<'static, 'static, ByNumber, u32, Record> {
        (index, Box::leak(Box::new(pk)))
    }

    fn encode_store_key(index: u32, pk: u32) -> Vec<u8> {
        store_key(index, pk).encode().as_ref().to_vec()
    }

    fn encode_index_prefix(index: u32) -> Vec<u8> {
        (index,).encode().as_ref().to_vec()
    }

    fn decode_store_key(bytes: Vec<u8>) -> StoreKey<'static, 'static, ByNumber, u32, Record> {
        let (index, rest) = u32::decode_part(&bytes);
        let (pk, _) = u32::decode_part(rest);
        store_key(index, pk)
    }
}
