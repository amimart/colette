use crate::store::{
    MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, WriteKVStore,
};

pub fn run_multistore_tests<DB: MultiStore>(make_db: impl Fn() -> DB) {
    basic_operations(&make_db);
    namespace_isolation(&make_db);
    store_isolation(&make_db);
    committed_writes_are_visible(&make_db);
    write_handle_reads_include_uncommitted_writes(&make_db);
    read_handles_keep_stable_snapshots(&make_db);
    multi_store_commits_are_atomic(&make_db);
}

fn basic_operations<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn namespace_isolation<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn store_isolation<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn committed_writes_are_visible<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn write_handle_reads_include_uncommitted_writes<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn read_handles_keep_stable_snapshots<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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

fn multi_store_commits_are_atomic<DB: MultiStore>(make_db: &impl Fn() -> DB) {
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
