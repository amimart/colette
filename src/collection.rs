use crate::entity::Entity;
use crate::error::Error;
use crate::index::{Index, IndexKind};
use crate::index_registry::{Cons, ContainsIndex, IndexRegistry, Nil};
use crate::key::Key;
use crate::scan::IndexScan;
use crate::store::{MultiStore, MultiStoreWriteHandle, ReadKVStore, WriteKVStore};
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

    pub fn builder<K, T>(name: &'static str, db: DB) -> CollectionBuilder<'static, DB, T, Nil>
    where
        K: Key,
        T: Entity,
    {
        CollectionBuilder::new(name, db)
    }

    pub fn insert(&self, value: Record) -> Result<(), Error> {
        let pk = value.key();
        let enc_pk = pk.encode();
        let mut tx = self.db.write(self.name)?;

        {
            let mut store = tx.open_store(Self::MAIN_STORE)?;

            if store.get(&enc_pk)?.is_some() {
                Err(Error::AlreadyExists(self.name.to_string()))?
            }

            store.set(&enc_pk, &value.to_bytes()?)?;
        }

        tx.commit().map_err(Error::Backend)
    }

    pub fn get(&self, _key: Record::Key<'_>) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    pub fn update(&self, _value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn save(&self, _value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn remove(&self, _key: Record::Key<'_>) -> Result<(), Error> {
        Ok(())
    }

    pub fn index<'a, Idx, P>(
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

pub struct CollectionBuilder<'a, DB, Record, Indexes>
where
    DB: MultiStore,
    Record: Entity + 'a,
    Indexes: IndexRegistry<'a, Record>,
{
    name: &'static str,
    db: DB,

    _marker: PhantomData<(&'a Record, Indexes)>,
}

impl<'a, DB, Record, Indexes> CollectionBuilder<'a, DB, Record, Indexes>
where
    DB: MultiStore,
    Record: Entity + 'a,
    Indexes: IndexRegistry<'a, Record>,
{
    pub fn new(name: &'static str, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    pub fn with_index<Idx>(self) -> CollectionBuilder<'a, DB, Record, Cons<Idx, Indexes>>
    where
        Idx: Index<'a, Record>,
    {
        assert!(
            !Indexes::has_index(Idx::NAME),
            "index with name '{}' already exists in collection '{}'",
            Idx::NAME,
            self.name
        );
        CollectionBuilder::new(self.name, self.db)
    }

    pub fn build(self) -> Collection<'a, DB, Record, Indexes> {
        Collection::new(self.name, self.db)
    }
}
