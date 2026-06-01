use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind};
use crate::index_registry::{Cons, ContainsIndex, IndexRegistry, Nil};
use crate::key::Key;
use crate::scan::IndexScan;
use crate::store::{
    MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, WriteKVStore,
};
use std::borrow::Borrow;
use std::marker::PhantomData;

pub struct Collection<DB, Record, Indexes>
where
    DB: MultiStore,
    // The stored record implementing the Entity contract
    Record: Entity,
    Indexes: IndexRegistry<Record>,
{
    name: &'static str,
    db: DB,

    _marker: PhantomData<(Record, Indexes)>,
}

impl<DB, Record, Indexes> Collection<DB, Record, Indexes>
where
    DB: MultiStore,
    Record: Entity,
    Indexes: IndexRegistry<Record>,
{
    const MAIN_STORE: &'static str = "__main";

    pub fn new(name: &'static str, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    /// Inserts a new record into the collection.
    ///
    /// Returns an error if a record with the same primary key already exists.
    ///
    /// All indexes are updated atomically within the same transaction.
    pub fn insert(&self, value: impl Borrow<Record>) -> Result<(), Error> {
        let value = value.borrow();
        let pk = value.key();
        let enc_pk = pk.encode();

        let mut tx = self.db.write(self.name)?;
        let mut store = tx.open_store(Self::MAIN_STORE)?;

        if store.get(&enc_pk)?.is_some() {
            Err(Error::AlreadyExists(format!("{:?}", pk)))?
        }

        store.set(&enc_pk, &value.to_bytes()?)?;

        Indexes::update(&mut tx, &pk, None, value)?;

        tx.commit().map_err(Error::Backend)
    }

    /// Updates an existing record in the collection.
    ///
    /// Returns an error if the record does not already exist.
    ///
    /// Indexes are automatically updated when indexed fields change.
    pub fn update(&self, value: impl Borrow<Record>) -> Result<(), Error> {
        let value = value.borrow();
        let pk = value.key();
        let enc_pk = pk.encode();

        let mut tx = self.db.write(self.name)?;
        let mut store = tx.open_store(Self::MAIN_STORE)?;

        let old = store
            .get(&enc_pk)?
            .map(|bytes| Record::from_bytes(&bytes).map_err(Error::Codec))
            .transpose()?;

        if old.is_none() {
            Err(Error::NotFound(format!("{:?}", pk)))?
        }

        store.set(&enc_pk, &value.to_bytes()?)?;

        Indexes::update(&mut tx, &pk, old.as_ref(), value)?;

        tx.commit().map_err(Error::Backend)
    }

    /// Saves a record into the collection.
    ///
    /// If the record already exists, it is updated.
    /// Otherwise, a new record is inserted.
    ///
    /// Indexes are updated atomically within the same transaction.
    pub fn save(&self, value: impl Borrow<Record>) -> Result<(), Error> {
        let value = value.borrow();
        let pk = value.key();
        let enc_pk = pk.encode();

        let mut tx = self.db.write(self.name)?;
        let mut store = tx.open_store(Self::MAIN_STORE)?;

        let old = store
            .get(&enc_pk)?
            .map(|bytes| Record::from_bytes(&bytes).map_err(Error::Codec))
            .transpose()?;

        store.set(&enc_pk, &value.to_bytes()?)?;

        Indexes::update(&mut tx, &pk, old.as_ref(), value)?;

        tx.commit().map_err(Error::Backend)
    }

    /// Removes a record from the collection by its primary key.
    ///
    /// If the record exists, all associated index entries are also removed.
    ///
    /// Returns `Ok(())` if the record does not exist.
    pub fn remove<'a>(
        &self,
        key: impl Borrow<<Record::Key<'a> as Key>::OwnedKey>,
    ) -> Result<(), Error>
    where
        Record: 'a,
    {
        let pk = key.borrow();
        let enc_pk = pk.encode();

        let mut tx = self.db.write(self.name)?;
        let mut store = tx.open_store(Self::MAIN_STORE)?;

        let record = store
            .get(enc_pk)?
            .map(|bytes| Record::from_bytes(&bytes).map_err(Error::Codec))
            .transpose()?;

        let record = match record {
            Some(record) => record,
            None => return Ok(()),
        };

        store.remove(key.borrow().encode())?;

        Indexes::remove(&mut tx, &record.key(), &record)?;

        tx.commit().map_err(Error::Backend)
    }

    /// Retrieves a record from the collection by its primary key.
    ///
    /// Returns `Ok(None)` if the record does not exist.
    pub fn get<'a>(
        &self,
        key: impl Borrow<<Record::Key<'a> as Key>::OwnedKey>,
    ) -> Result<Option<Record>, Error>
    where
        Record: 'a,
    {
        self.db
            .read(self.name)?
            .open_store(Self::MAIN_STORE)?
            .get(key.borrow().encode())?
            .map(|bytes| Record::from_bytes(&bytes).map_err(Error::Codec))
            .transpose()
    }

    /// Creates a typed scan over a collection index.
    ///
    /// Scans can be configured with:
    /// - prefixes
    /// - cursors
    /// - ordering direction
    /// - limits
    ///
    /// The returned scan is lazy and does not perform any database access
    /// until iterated.
    pub fn scan<'a, Idx, P>(
        &self,
        _idx: Idx,
    ) -> Result<IndexScan<'a, DB::ReadHandle, Record, Idx>, Error>
    where
        Idx: Index<Record>,
        Idx::Kind<'a>: IndexKind<Idx::Key<'a>, Record::Key<'a>>,
        Indexes: ContainsIndex<Idx, P>,
    {
        Ok(IndexScan::new(self.name, self.db.read(self.name)?))
    }
}

pub struct CollectionBuilder<DB, Record, Indexes>
where
    DB: MultiStore,
    Record: Entity,
    Indexes: IndexRegistry<Record>,
{
    name: &'static str,
    db: DB,

    _marker: PhantomData<(Record, Indexes)>,
}

impl<DB, Record, Indexes> CollectionBuilder<DB, Record, Indexes>
where
    DB: MultiStore,
    Record: Entity,
    Indexes: IndexRegistry<Record>,
{
    pub fn with_index<Idx>(self) -> CollectionBuilder<DB, Record, Cons<Idx, Indexes>>
    where
        Idx: Index<Record>,
        for<'ik, 'pk> Idx::Kind<'ik>: IndexKind<Idx::Key<'ik>, Record::Key<'pk>>,
    {
        assert!(
            !Indexes::has_index(Idx::NAME),
            "index with name '{}' already exists in collection '{}'",
            Idx::NAME,
            self.name
        );
        CollectionBuilder {
            name: self.name,
            db: self.db,
            _marker: PhantomData,
        }
    }

    pub fn build(self) -> Collection<DB, Record, Indexes> {
        Collection::new(self.name, self.db)
    }
}

pub fn collection<T, DB>(name: &'static str, db: DB) -> CollectionBuilder<DB, T, Nil>
where
    T: Entity,
    DB: MultiStore,
{
    CollectionBuilder {
        name,
        db,
        _marker: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::collection::Collection;
    use crate::entity::Entity;
    use crate::error::{CodecError, Error};
    use crate::index::{Index, Unique};
    use crate::index_registry::{Cons, Nil};
    use crate::key::Key;
    use crate::store::MultiStoreWriteHandle;
    use crate::testing::{MockDb, backend_error};

    // ── Minimal entity ────────────────────────────────────────────────────────

    struct TestRecord {
        id: u32,
    }

    impl Entity for TestRecord {
        type Key<'a> = u32;

        fn key(&self) -> u32 {
            self.id
        }

        fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
            Ok(self.id.to_be_bytes().to_vec())
        }

        fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
            let id = u32::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| CodecError::new(std::io::Error::other("bad length")))?,
            );
            Ok(TestRecord { id })
        }
    }

    // ── Spy indexes ───────────────────────────────────────────────────────────
    // Each spy opens its named store (observable via TxLog::opens) and succeeds.

    struct IndexA;
    struct IndexB;
    struct FailIndex;

    macro_rules! spy_index {
        ($ty:ty, $name:literal) => {
            impl Index<TestRecord> for $ty {
                type Key<'a> = u32;
                type Kind<'a> = Unique;
                const NAME: &'static str = $name;

                fn key(r: &TestRecord) -> u32 {
                    r.id
                }

                fn update<DB: MultiStoreWriteHandle>(
                    db: &mut DB,
                    _pk: &u32,
                    _old: Option<&TestRecord>,
                    _new: &TestRecord,
                ) -> Result<(), Error> {
                    db.open_store(Self::NAME)?;
                    Ok(())
                }

                fn remove<DB: MultiStoreWriteHandle>(
                    db: &mut DB,
                    _pk: &u32,
                    _item: &TestRecord,
                ) -> Result<(), Error> {
                    db.open_store(Self::NAME)?;
                    Ok(())
                }
            }
        };
    }

    spy_index!(IndexA, "index_a");
    spy_index!(IndexB, "index_b");

    impl Index<TestRecord> for FailIndex {
        type Key<'a> = u32;
        type Kind<'a> = Unique;
        const NAME: &'static str = "fail";

        fn key(r: &TestRecord) -> u32 {
            r.id
        }

        fn update<DB: MultiStoreWriteHandle>(
            _db: &mut DB,
            _pk: &u32,
            _old: Option<&TestRecord>,
            _new: &TestRecord,
        ) -> Result<(), Error> {
            Err(Error::Unexpected("injected index error".into()))
        }

        fn remove<DB: MultiStoreWriteHandle>(
            _db: &mut DB,
            _pk: &u32,
            _item: &TestRecord,
        ) -> Result<(), Error> {
            Err(Error::Unexpected("injected index error".into()))
        }
    }

    // ── insert ────────────────────────────────────────────────────────────────

    #[test]
    fn insert() {
        let enc_pk = 1u32.encode().to_vec();
        let enc_val = TestRecord { id: 1 }.to_bytes().unwrap();

        // Helper macro: runs insert with the given Indexes type parameter (which is
        // a compile-time choice) and returns the result + a snapshot of the log.
        macro_rules! run {
            ($indexes:ty, $db:expr) => {{
                let db: MockDb = $db;
                let log = db.log();
                let col = Collection::<_, TestRecord, $indexes>::new("col", db);
                let result = col.insert(TestRecord { id: 1 });
                let log = Rc::clone(&log);
                (result, log)
            }};
        }

        // ── Store-level cases (index type fixed to Nil) ───────────────────────

        struct Case {
            name: &'static str,
            db: MockDb,
            expect_result: fn(&Result<(), Error>),
            expect_opens: &'static [&'static str],
            expect_sets: usize,
            expect_committed: bool,
        }

        let cases = vec![
            Case {
                name: "inserts new record",
                db: MockDb::new(),
                expect_result: |r| assert!(r.is_ok()),
                expect_opens: &["__main"],
                expect_sets: 1,
                expect_committed: true,
            },
            Case {
                name: "fails when record already exists",
                db: MockDb::new().with_data("__main", enc_pk.clone(), enc_val.clone()),
                expect_result: |r| assert!(matches!(r, Err(Error::AlreadyExists(_)))),
                expect_opens: &["__main"],
                expect_sets: 0,
                expect_committed: false,
            },
            Case {
                name: "propagates backend error from write()",
                db: MockDb::new().with_write_err(|| backend_error("write failed")),
                expect_result: |r| assert!(matches!(r, Err(Error::Backend(_)))),
                expect_opens: &[],
                expect_sets: 0,
                expect_committed: false,
            },
            Case {
                // set is called before commit; commit failure must be propagated
                name: "propagates backend error from commit()",
                db: MockDb::new().with_commit_err(|| backend_error("commit failed")),
                expect_result: |r| assert!(matches!(r, Err(Error::Backend(_)))),
                expect_opens: &["__main"],
                expect_sets: 1,
                expect_committed: true,
            },
        ];

        for c in cases {
            let (result, log) = run!(Nil, c.db);
            let log = log.borrow();

            (c.expect_result)(&result);
            assert_eq!(log.opens.as_slice(), c.expect_opens, "[{}] opens", c.name);
            assert_eq!(log.sets.len(), c.expect_sets, "[{}] sets count", c.name);
            assert_eq!(log.committed, c.expect_committed, "[{}] committed", c.name);
        }

        // Verify set writes the correct key and value bytes
        let (result, log) = run!(Nil, MockDb::new());
        assert!(result.is_ok());
        let log = log.borrow();
        assert_eq!(log.sets[0].0, enc_pk, "set key must be the encoded primary key");
        assert_eq!(log.sets[0].1, enc_val, "set value must be to_bytes() output");

        // ── Index dispatch cases ──────────────────────────────────────────────

        struct IndexCase {
            name: &'static str,
            expected_opens: &'static [&'static str],
            expect_ok: bool,
            expect_committed: bool,
        }

        // Indexes vary at compile time, so we use the macro for each arity.
        let (r, log) = run!(Nil, MockDb::new());
        let nil_case = (r, log.borrow().opens.clone(), log.borrow().committed);

        let (r, log) = run!(Cons<IndexA, Nil>, MockDb::new());
        let one_index = (r, log.borrow().opens.clone(), log.borrow().committed);

        let (r, log) = run!(Cons<IndexA, Cons<IndexB, Nil>>, MockDb::new());
        let two_indexes = (r, log.borrow().opens.clone(), log.borrow().committed);

        // Failing index: set is still called on main store, but commit is skipped
        let (r, log) = run!(Cons<FailIndex, Nil>, MockDb::new());
        let fail_index = (r, log.borrow().opens.clone(), log.borrow().committed, log.borrow().sets.len());

        let index_cases: &[IndexCase] = &[
            IndexCase {
                name: "no indexes: only main store opened",
                expected_opens: &["__main"],
                expect_ok: true, expect_committed: true,
            },
            IndexCase {
                name: "one index: main store then index store opened",
                expected_opens: &["__main", "index_a"],
                expect_ok: true, expect_committed: true,
            },
            IndexCase {
                name: "two indexes: all stores opened in registry order",
                expected_opens: &["__main", "index_a", "index_b"],
                expect_ok: true, expect_committed: true,
            },
        ];

        let dispatch_results = [nil_case, one_index, two_indexes];
        for (c, (result, opens, committed)) in index_cases.iter().zip(&dispatch_results) {
            assert_eq!(result.is_ok(), c.expect_ok, "[{}] result", c.name);
            assert_eq!(opens.as_slice(), c.expected_opens, "[{}] opens", c.name);
            assert_eq!(*committed, c.expect_committed, "[{}] committed", c.name);
        }

        // Failing index: error propagated, set occurred, commit skipped
        let (fail_result, fail_opens, fail_committed, fail_sets) = fail_index;
        assert!(matches!(fail_result, Err(Error::Unexpected(_))), "failing index error propagated");
        assert_eq!(fail_opens.as_slice(), &["__main"] as &[&str], "failing index: only main store opened");
        assert_eq!(fail_sets, 1, "failing index: main record was written before index error");
        assert!(!fail_committed, "failing index: commit must not be called");
    }
}
