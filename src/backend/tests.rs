use crate::store::{
    MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, WriteKVStore,
};

pub fn run_multistore_tests<DB: MultiStore>(make_db: impl Fn() -> DB) {
    basic_operations(&make_db);
    namespace_isolation(&make_db);
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
