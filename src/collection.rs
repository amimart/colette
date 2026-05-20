use std::marker::PhantomData;
use crate::entity::Entity;
use crate::store::{MultiStore};
use crate::error::Error;
use crate::key::Key;

pub struct Collection<DB, PrimaryKey, Record>
where
    DB: MultiStore,
    PrimaryKey: Key,
    // The stored record implementing the Entity contract
    Record: Entity<PrimaryKey>,
{
    name: String,
    db: DB,

    _marker: PhantomData<(PrimaryKey, Record)>,
}

impl<DB, PrimaryKey, Record, Indexes> Collection<DB, PrimaryKey, Record>
where
    DB: MultiStore,
    PrimaryKey: Key,
    Record: Entity<PrimaryKey>,
{
    pub fn new(name: String, db: DB) -> Self {
        Self {
            name,
            db,
            _marker: PhantomData,
        }
    }

    pub fn insert(&self, value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn get(&self, key: PrimaryKey) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    pub fn update(&self, value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn save(&self, value: Record) -> Result<(), Error> {
        Ok(())
    }

    pub fn remove(&self, key: PrimaryKey) -> Result<(), Error> {
        Ok(())
    }
}
