use crate::scan::{Direction, ScanRange};
use crate::store::{
    MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, WriteKVStore,
};
use std::ops::Bound;

#[allow(dead_code)]
pub fn run_multistore_tests<DB: MultiStore>(make_db: impl Fn() -> DB) {
    basic_operations(&make_db);
    namespace_isolation(&make_db);
    store_isolation(&make_db);
    committed_writes_are_visible(&make_db);
    write_handle_reads_include_uncommitted_writes(&make_db);
    read_handles_keep_stable_snapshots(&make_db);
    multi_store_commits_are_atomic(&make_db);
    scans(&make_db);
}

macro_rules! multistore_contract_tests {
    ($make_db:expr) => {
        mod contract {
            use super::*;

            #[test]
            fn basic_operations() {
                $crate::backend::tests::basic_operations(&$make_db);
            }

            #[test]
            fn namespace_isolation() {
                $crate::backend::tests::namespace_isolation(&$make_db);
            }

            #[test]
            fn store_isolation() {
                $crate::backend::tests::store_isolation(&$make_db);
            }

            #[test]
            fn committed_writes_are_visible() {
                $crate::backend::tests::committed_writes_are_visible(&$make_db);
            }

            #[test]
            fn write_handle_reads_include_uncommitted_writes() {
                $crate::backend::tests::write_handle_reads_include_uncommitted_writes(&$make_db);
            }

            #[test]
            fn read_handles_keep_stable_snapshots() {
                $crate::backend::tests::read_handles_keep_stable_snapshots(&$make_db);
            }

            #[test]
            fn multi_store_commits_are_atomic() {
                $crate::backend::tests::multi_store_commits_are_atomic(&$make_db);
            }

            #[test]
            fn scans() {
                $crate::backend::tests::scans(&$make_db);
            }
        }
    };
}

pub(crate) use multistore_contract_tests;

pub(crate) fn basic_operations<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("basic", ["items"]).unwrap();

    assert_eq!(get(&db, "basic", "items", b"missing"), None);

    commit_entries(&db, "basic", "items", &[(b"a".to_vec(), b"one".to_vec())]);
    assert_eq!(get(&db, "basic", "items", b"a"), Some(b"one".to_vec()));

    commit_entries(&db, "basic", "items", &[(b"a".to_vec(), b"two".to_vec())]);
    assert_eq!(get(&db, "basic", "items", b"a"), Some(b"two".to_vec()));

    remove_and_commit(&db, "basic", "items", b"a");
    assert_eq!(get(&db, "basic", "items", b"a"), None);

    remove_and_commit(&db, "basic", "items", b"missing");
    assert_eq!(get(&db, "basic", "items", b"missing"), None);
}

pub(crate) fn namespace_isolation<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("left", ["items"]).unwrap();
    db.prepare("right", ["items"]).unwrap();

    commit_entries(
        &db,
        "left",
        "items",
        &[(b"same-key".to_vec(), b"left".to_vec())],
    );
    commit_entries(
        &db,
        "right",
        "items",
        &[(b"same-key".to_vec(), b"right".to_vec())],
    );

    assert_eq!(
        get(&db, "left", "items", b"same-key"),
        Some(b"left".to_vec())
    );
    assert_eq!(
        get(&db, "right", "items", b"same-key"),
        Some(b"right".to_vec())
    );
}

pub(crate) fn store_isolation<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("stores", ["primary", "index"]).unwrap();

    {
        let mut write = db.write("stores").unwrap();
        {
            let mut primary = write.open_store("primary").unwrap();
            primary.set(b"same-key", b"primary").unwrap();
        }
        {
            let mut index = write.open_store("index").unwrap();
            index.set(b"same-key", b"index").unwrap();
        }
        write.commit().unwrap();
    }

    assert_eq!(
        get(&db, "stores", "primary", b"same-key"),
        Some(b"primary".to_vec())
    );
    assert_eq!(
        get(&db, "stores", "index", b"same-key"),
        Some(b"index".to_vec())
    );
}

pub(crate) fn committed_writes_are_visible<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("commits", ["items"]).unwrap();

    commit_entries(
        &db,
        "commits",
        "items",
        &[(b"k".to_vec(), b"committed".to_vec())],
    );

    assert_eq!(
        get(&db, "commits", "items", b"k"),
        Some(b"committed".to_vec())
    );
}

pub(crate) fn write_handle_reads_include_uncommitted_writes<DB: MultiStore>(
    make_db: &impl Fn() -> DB,
) {
    let db = make_db();
    db.prepare("write-reads", ["items"]).unwrap();

    let mut write = db.write("write-reads").unwrap();
    {
        let mut store = write.open_store("items").unwrap();
        store.set(b"k", b"uncommitted").unwrap();
        assert_eq!(store.get(b"k").unwrap(), Some(b"uncommitted".to_vec()));
    }
    write.commit().unwrap();
}

pub(crate) fn read_handles_keep_stable_snapshots<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("snapshots", ["items"]).unwrap();

    let before = db.read("snapshots").unwrap();

    commit_entries(
        &db,
        "snapshots",
        "items",
        &[(b"k".to_vec(), b"after".to_vec())],
    );

    assert_eq!(
        before.open_store("items").unwrap().get(b"k").unwrap(),
        None,
        "existing read handles should keep their pre-commit snapshot"
    );
    assert_eq!(
        get(&db, "snapshots", "items", b"k"),
        Some(b"after".to_vec())
    );
}

pub(crate) fn multi_store_commits_are_atomic<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("atomic", ["primary", "index"]).unwrap();

    let before = db.read("atomic").unwrap();

    let mut write = db.write("atomic").unwrap();
    {
        let mut primary = write.open_store("primary").unwrap();
        primary.set(b"id:1", b"record").unwrap();
    }
    {
        let mut index = write.open_store("index").unwrap();
        index.set(b"name:record", b"id:1").unwrap();
    }

    assert_eq!(
        before.open_store("primary").unwrap().get(b"id:1").unwrap(),
        None
    );
    assert_eq!(
        before
            .open_store("index")
            .unwrap()
            .get(b"name:record")
            .unwrap(),
        None
    );

    write.commit().unwrap();

    assert_eq!(
        get(&db, "atomic", "primary", b"id:1"),
        Some(b"record".to_vec())
    );
    assert_eq!(
        get(&db, "atomic", "index", b"name:record"),
        Some(b"id:1".to_vec())
    );
}

pub(crate) fn scans<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let db = make_db();
    db.prepare("scans", ["items"]).unwrap();
    commit_entries(&db, "scans", "items", &scan_entries());

    let cases = vec![
        ScanCase::new(
            "full scan left-to-right",
            ScanRange::All,
            Direction::LeftToRight,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        ScanCase::new(
            "full scan right-to-left",
            ScanRange::All,
            Direction::RightToLeft,
            &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        ),
        ScanCase::new(
            "inclusive lower bound",
            ScanRange::Range {
                left: Bound::Included(v(&[1])),
                right: Bound::Unbounded,
            },
            Direction::LeftToRight,
            &[4, 5, 6, 7, 8, 9, 10, 11],
        ),
        ScanCase::new(
            "exclusive lower bound",
            ScanRange::Range {
                left: Bound::Excluded(v(&[1])),
                right: Bound::Unbounded,
            },
            Direction::LeftToRight,
            &[5, 6, 7, 8, 9, 10, 11],
        ),
        ScanCase::new(
            "inclusive upper bound",
            ScanRange::Range {
                left: Bound::Unbounded,
                right: Bound::Included(v(&[1])),
            },
            Direction::LeftToRight,
            &[0, 1, 2, 3, 4],
        ),
        ScanCase::new(
            "exclusive upper bound",
            ScanRange::Range {
                left: Bound::Unbounded,
                right: Bound::Excluded(v(&[1])),
            },
            Direction::LeftToRight,
            &[0, 1, 2, 3],
        ),
        ScanCase::new(
            "bounded inclusive/inclusive",
            ScanRange::Range {
                left: Bound::Included(v(&[0, 1])),
                right: Bound::Included(v(&[2])),
            },
            Direction::LeftToRight,
            &[3, 4, 5, 6],
        ),
        ScanCase::new(
            "bounded inclusive/exclusive",
            ScanRange::Range {
                left: Bound::Included(v(&[0, 1])),
                right: Bound::Excluded(v(&[2])),
            },
            Direction::LeftToRight,
            &[3, 4, 5],
        ),
        ScanCase::new(
            "bounded exclusive/inclusive",
            ScanRange::Range {
                left: Bound::Excluded(v(&[0, 1])),
                right: Bound::Included(v(&[2])),
            },
            Direction::LeftToRight,
            &[4, 5, 6],
        ),
        ScanCase::new(
            "bounded exclusive/exclusive",
            ScanRange::Range {
                left: Bound::Excluded(v(&[0, 1])),
                right: Bound::Excluded(v(&[2])),
            },
            Direction::LeftToRight,
            &[4, 5],
        ),
        ScanCase::new(
            "empty range",
            ScanRange::Range {
                left: Bound::Included(v(&[4])),
                right: Bound::Included(v(&[5])),
            },
            Direction::LeftToRight,
            &[],
        ),
        ScanCase::new(
            "single-item range",
            ScanRange::Range {
                left: Bound::Included(v(&[1, 0])),
                right: Bound::Included(v(&[1, 0])),
            },
            Direction::LeftToRight,
            &[5],
        ),
        ScanCase::new(
            "reverse scan with bounds",
            ScanRange::Range {
                left: Bound::Included(v(&[0, 1])),
                right: Bound::Included(v(&[2])),
            },
            Direction::RightToLeft,
            &[6, 5, 4, 3],
        ),
        ScanCase::new(
            "empty prefix left-to-right",
            ScanRange::Prefix(v(&[])),
            Direction::LeftToRight,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        ScanCase::new(
            "empty prefix right-to-left",
            ScanRange::Prefix(v(&[])),
            Direction::RightToLeft,
            &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        ),
        ScanCase::new(
            "prefix with exact key and descendants",
            ScanRange::Prefix(v(&[0])),
            Direction::LeftToRight,
            &[1, 2, 3],
        ),
        ScanCase::new(
            "reverse prefix with exact key and descendants",
            ScanRange::Prefix(v(&[1])),
            Direction::RightToLeft,
            &[5, 4],
        ),
        ScanCase::new(
            "prefix with descendants but no exact key",
            ScanRange::Prefix(v(&[3])),
            Direction::LeftToRight,
            &[7, 8],
        ),
        ScanCase::new(
            "reverse prefix with descendants but no exact key",
            ScanRange::Prefix(v(&[3])),
            Direction::RightToLeft,
            &[8, 7],
        ),
        ScanCase::new(
            "single-item prefix",
            ScanRange::Prefix(v(&[1, 0])),
            Direction::LeftToRight,
            &[5],
        ),
        ScanCase::new(
            "prefix without finite upper bound",
            ScanRange::Prefix(v(&[255])),
            Direction::LeftToRight,
            &[10, 11],
        ),
        ScanCase::new(
            "reverse prefix without finite upper bound",
            ScanRange::Prefix(v(&[255])),
            Direction::RightToLeft,
            &[11, 10],
        ),
        ScanCase::new(
            "missing prefix",
            ScanRange::Prefix(v(&[4])),
            Direction::LeftToRight,
            &[],
        ),
        ScanCase::new(
            "lexicographic byte ordering",
            ScanRange::Range {
                left: Bound::Included(v(&[2])),
                right: Bound::Included(v(&[10])),
            },
            Direction::LeftToRight,
            &[6, 7, 8, 9],
        ),
    ];

    for case in cases {
        assert_scan(&db, case);
    }

    remove_and_commit(&db, "scans", "items", &[1, 0]);

    for case in [ScanCase::new(
        "scan after remove",
        ScanRange::All,
        Direction::LeftToRight,
        &[0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11],
    )] {
        assert_scan(&db, case);
    }
}

fn commit_entries<DB: MultiStore>(
    db: &DB,
    namespace: &'static str,
    store: &'static str,
    entries: &[(Vec<u8>, Vec<u8>)],
) {
    let mut write = db.write(namespace).unwrap();
    {
        let mut store = write.open_store(store).unwrap();
        for (key, value) in entries {
            store.set(key, value).unwrap();
        }
    }
    write.commit().unwrap();
}

fn remove_and_commit<DB: MultiStore>(
    db: &DB,
    namespace: &'static str,
    store: &'static str,
    key: impl AsRef<[u8]>,
) {
    let mut write = db.write(namespace).unwrap();
    {
        let mut store = write.open_store(store).unwrap();
        store.remove(key).unwrap();
    }
    write.commit().unwrap();
}

fn get<DB: MultiStore>(
    db: &DB,
    namespace: &'static str,
    store: &'static str,
    key: impl AsRef<[u8]>,
) -> Option<Vec<u8>> {
    db.read(namespace)
        .unwrap()
        .open_store(store)
        .unwrap()
        .get(key)
        .unwrap()
}

struct ScanCase {
    name: &'static str,
    range: ScanRange,
    direction: Direction,
    expected_indexes: &'static [usize],
}

impl ScanCase {
    fn new(
        name: &'static str,
        range: ScanRange,
        direction: Direction,
        expected_indexes: &'static [usize],
    ) -> Self {
        Self {
            name,
            range,
            direction,
            expected_indexes,
        }
    }
}

fn assert_scan<DB: MultiStore>(db: &DB, case: ScanCase) {
    let expected = case
        .expected_indexes
        .into_iter()
        .map(|index| scan_entries()[*index].clone())
        .collect::<Vec<_>>();

    assert_eq!(
        scan(db, "scans", "items", case.range, case.direction),
        expected,
        "{}",
        case.name,
    );
}

fn scan<DB: MultiStore>(
    db: &DB,
    namespace: &'static str,
    store: &'static str,
    range: ScanRange,
    direction: Direction,
) -> Vec<(Vec<u8>, Vec<u8>)> {
    db.read(namespace)
        .unwrap()
        .open_store(store)
        .unwrap()
        .scan(range, direction)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn scan_entries() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (v(&[]), b"empty".to_vec()),
        (v(&[0]), b"zero".to_vec()),
        (v(&[0, 0]), b"zero-zero".to_vec()),
        (v(&[0, 1]), b"zero-one".to_vec()),
        (v(&[1]), b"one".to_vec()),
        (v(&[1, 0]), b"one-zero".to_vec()),
        (v(&[2]), b"two".to_vec()),
        (v(&[3, 0]), b"three-zero".to_vec()),
        (v(&[3, 1]), b"three-one".to_vec()),
        (v(&[10]), b"ten".to_vec()),
        (v(&[255]), b"max".to_vec()),
        (v(&[255, 0]), b"max-zero".to_vec()),
    ]
}

fn v(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
