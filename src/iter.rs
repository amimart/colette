use crate::entity::Entity;
use crate::error::{BackendError, Error};
use crate::store::ReadKVStore;
use std::marker::PhantomData;

pub struct IndexEntry<Record> {
    pub record: Record,
    pub key: Cursor,
}

#[allow(dead_code)]
pub struct Cursor(Vec<u8>);

pub struct IndexIterator<'a, Store, Record>
where
    Store: ReadKVStore,
    Record: Entity,
    Store: 'a,
{
    inner: Store::Iter<'a>,
    primary_store: Store,

    _marker: PhantomData<Record>,
}

impl<'a, Store, Record> IndexIterator<'a, Store, Record>
where
    Store: ReadKVStore,
    Record: Entity,
{
    pub fn new(inner: Store::Iter<'a>, primary_store: Store) -> Self {
        Self {
            inner,
            primary_store,

            _marker: PhantomData,
        }
    }
}

impl<'a, Store, Record> Iterator for IndexIterator<'a, Store, Record>
where
    Store: ReadKVStore,
    Record: Entity,
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
