use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind};
use crate::index_registry::{Cons, ContainsIndex, IndexRegistry, Nil};
use crate::key::Key;
use crate::scan::IndexScan;
use crate::store::{MultiStore, MultiStoreWriteHandle, ReadKVStore, WriteKVStore};
use std::marker::PhantomData;

pub struct Collection<DB, PrimaryKey, Record, Indexes>
where
    DB: MultiStore,
    PrimaryKey: Key,
    // The stored record implementing the Entity contract
    Record: Entity<PrimaryKey>,
    Indexes: IndexRegistry<PrimaryKey, Record>,
{
    name: &'static str,
    db: DB,

    _marker: PhantomData<(PrimaryKey, Record, Indexes)>,
}

impl<DB, PrimaryKey, Record, Indexes> Collection<DB, PrimaryKey, Record, Indexes>
where
    DB: MultiStore,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Indexes: IndexRegistry<PrimaryKey, Record>,
{
    const MAIN_STORE: &'static str = "__main";

    pub fn new(name: &'static str, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    pub fn builder<K, T>(name: &'static str, db: DB) -> CollectionBuilder<DB, K, T, Nil>
    where
        K: Key,
        T: Entity<K>,
    {
        CollectionBuilder::new(name, db)
    }

    pub fn insert(&self, value: Record) -> Result<(), Error> {
        let pk = value.key().encode();
        let mut tx = self.db.write(self.name)?;

        {
            let mut store = tx.open_store(Self::MAIN_STORE)?;

            if store.get(&pk)?.is_some() {
                Err(Error::AlreadyExists(self.name.to_string()))?
            }

            store.set(&pk, &value.to_bytes()?)?;
        }

        tx.commit().map_err(Error::Backend)
    }

    pub fn get(&self, _key: PrimaryKey) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    pub fn update(&self, _value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn save(&self, _value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn remove(&self, _key: PrimaryKey) -> Result<(), Error> {
        Ok(())
    }

    pub fn index<Idx, P>(
        &self,
        _idx: Idx,
    ) -> Result<IndexScan<'_, DB::ReadHandle, PrimaryKey, Record, Idx>, Error>
    where
        Idx: Index<PrimaryKey, Record>,
        Idx::Kind: IndexKind<Idx::Key, PrimaryKey>,
        Indexes: ContainsIndex<Idx, P>,
    {
        Ok(IndexScan::new(self.name, self.db.read(self.name)?))
    }
}

pub struct CollectionBuilder<DB, PrimaryKey, Record, Indexes>
where
    DB: MultiStore,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Indexes: IndexRegistry<PrimaryKey, Record>,
{
    name: &'static str,
    db: DB,

    _marker: PhantomData<(PrimaryKey, Record, Indexes)>,
}

impl<DB, PrimaryKey, Record, Indexes> CollectionBuilder<DB, PrimaryKey, Record, Indexes>
where
    DB: MultiStore,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
    Indexes: IndexRegistry<PrimaryKey, Record>,
{
    pub fn new(name: &'static str, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    pub fn with_index<Idx>(self) -> CollectionBuilder<DB, PrimaryKey, Record, Cons<Idx, Indexes>>
    where
        Idx: Index<PrimaryKey, Record>,
    {
        assert!(
            !Indexes::has_index(Idx::NAME),
            "index with name '{}' already exists in collection '{}'",
            Idx::NAME,
            self.name
        );
        CollectionBuilder::new(self.name, self.db)
    }

    pub fn build(self) -> Collection<DB, PrimaryKey, Record, Indexes> {
        Collection::new(self.name, self.db)
    }
}
