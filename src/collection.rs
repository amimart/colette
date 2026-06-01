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

    pub fn save(&self, _value: impl Borrow<Record>) -> Result<(), Error> {
        Ok(())
    }

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
