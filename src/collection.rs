use crate::entity::Entity;
use crate::error::Error;
use crate::index::{ContainsIndex, Index, IndexKind, IndexRegistry};
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
    name: String,
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
    pub fn new(name: String, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    pub fn insert(&self, value: Record) -> Result<(), Error> {
        let pk = value.key().encode();
        let mut tx = self.db.write()?;

        {
            let mut store = tx.open_store(&self.name)?;

            if store.get(&pk)?.is_some() {
                Err(Error::AlreadyExists(self.name.clone()))?
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
        Ok(IndexScan::new(self.name.clone(), self.db.read()?))
    }
}
