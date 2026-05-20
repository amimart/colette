use crate::entity::Entity;
use crate::error::Error;
use crate::key::Key;
use crate::store::ReadKVStore;
use std::marker::PhantomData;

pub struct IndexEntry<Record> {
    pub record: Record,
    pub key: Cursor,
}

pub struct Cursor(Vec<u8>);

pub struct IndexIterator<Store, PrimaryKey, Record>
where
    Store: ReadKVStore,
    Record: Entity<PrimaryKey>,
    PrimaryKey: Key,
{
    inner: Store::Iter,
    primary_store: Store,

    _marker: PhantomData<(PrimaryKey, Record)>,
}

impl<Store, PrimaryKey, Record> IndexIterator<Store, PrimaryKey, Record>
where
    Store: ReadKVStore,
    Record: Entity<PrimaryKey>,
    PrimaryKey: Key,
{
    pub fn new(inner: Store::Iter, primary_store: Store) -> Self {
        Self {
            inner,
            primary_store,

            _marker: PhantomData,
        }
    }
}

impl<Store, PrimaryKey, Record> Iterator for IndexIterator<Store, PrimaryKey, Record>
where
    Store: ReadKVStore,
    Record: Entity<PrimaryKey>,
    PrimaryKey: Key,
{
    type Item = Result<IndexEntry<Record>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map_err(Error::Backend).and_then(|(cursor, pk)| {
                let record_bytes =
                    self.primary_store
                        .get(&pk)?
                        .ok_or(Error::Unexpected(format!(
                            "primary key from index not found: {:?}",
                            pk
                        )))?;
                let record = Record::from_bytes(&record_bytes)?;

                Ok(IndexEntry {
                    record,
                    key: Cursor(cursor),
                })
            })
        })
    }
}
