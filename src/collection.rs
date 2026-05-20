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
        let pk = value.key().encode();
        let mut tx = self.db.write()?;

        {
            let mut store = tx.open_store(&self.name)?;

            if let Some(_) = store.get(&pk)? {
                Err(Error::AlreadyExists(self.name.clone()))?
            }

            store.set(&pk, &value.to_bytes()?)?;
        }

        tx.commit().map_err(|e| Error::Backend(e.into()))
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
