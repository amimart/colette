use crate::entity::Entity;
use crate::error::Error;
use crate::key::{AppendKey, Key};
use crate::store::{MultiStoreWriteHandle, ReadKVStore, WriteKVStore};

/// Index allows to maintain a separate query efficient stores on non primary-key, it is made for
/// a specific Entity and specified by a Key to index extracted from an Entity, and an IndexKind
/// (i.e. Unique or Multi).
pub trait Index<Record: Entity> {
    type Key<'a>: Key
    where
        Record: 'a;

    type Kind<'a>: IndexKind<Self::Key<'a>, Record::Key<'a>>
    where
        Record: 'a;

    const NAME: &'static str;

    fn key(entity: &Record) -> Self::Key<'_>;

    fn update<'a, DB: MultiStoreWriteHandle>(
        db: &mut DB,
        pk: &Record::Key<'a>,
        old: Option<&Record>,
        new: &'a Record,
    ) -> Result<(), Error>
    where
        for<'ik, 'pk> Self::Kind<'ik>: IndexKind<Self::Key<'ik>, Record::Key<'pk>>,
    {
        let new_skey = Self::Kind::store_key(Self::key(new), pk);

        let mut store = db.open_store(Self::NAME)?;
        if let Some(entity) = old {
            let old_skey = Self::Kind::store_key(Self::key(entity), pk);

            if old_skey == new_skey {
                return Ok(());
            }

            store.remove(old_skey.encode())?;
        }

        let skey = new_skey.encode();
        if store.get(&skey)?.is_some() {
            Err(Error::AlreadyExists(format!("{:?}", new_skey)))?
        }

        store.set(skey, pk.encode())?;

        Ok(())
    }

    fn remove<'a, DB: MultiStoreWriteHandle>(
        db: &mut DB,
        pk: &Record::Key<'a>,
        item: &'a Record,
    ) -> Result<(), Error> {
        let mut store = db.open_store(Self::NAME)?;
        let skey = Self::Kind::store_key(Self::key(item), pk);

        store.remove(skey.encode()).map_err(Error::Backend)
    }
}

pub type StoreKey<'a, 'b, I, PK, T> =
    <<I as Index<T>>::Kind<'a> as IndexKind<<I as Index<T>>::Key<'b>, PK>>::StoreKey<'a, 'b>;

/// IndexKind helps to specify an index behavior by expressing the actual stored key in the index
/// based on the index key and the underlying entity primary key.
///
/// For example, a unique index can store only the index key as the store key, while a multi index
/// needs to append the primary key to the index key to guarantee the uniqueness of each entry.
pub trait IndexKind<IndexKey, PrimaryKey>
where
    IndexKey: Key,
    PrimaryKey: Key,
{
    type StoreKey<'a, 'b>: Key
    where
        IndexKey: 'a,
        PrimaryKey: 'b;

    fn store_key<'a, 'b>(k: IndexKey, pk: &'b PrimaryKey) -> Self::StoreKey<'a, 'b>
    where
        IndexKey: 'a;
}

pub struct Unique;

impl<IndexKey, PrimaryKey> IndexKind<IndexKey, PrimaryKey> for Unique
where
    IndexKey: Key,
    PrimaryKey: Key,
{
    type StoreKey<'a, 'b>
        = IndexKey
    where
        IndexKey: 'a,
        PrimaryKey: 'b;

    fn store_key<'a, 'b>(k: IndexKey, _pk: &'b PrimaryKey) -> Self::StoreKey<'a, 'b>
    where
        IndexKey: 'a,
    {
        k
    }
}

pub struct Multi;

impl<IndexKey, PrimaryKey> IndexKind<IndexKey, PrimaryKey> for Multi
where
    IndexKey: Key + AppendKey<PrimaryKey>,
    PrimaryKey: Key,
{
    type StoreKey<'a, 'b>
        = <IndexKey as AppendKey<PrimaryKey>>::Key<'b>
    where
        IndexKey: 'a,
        PrimaryKey: 'b;

    fn store_key<'a, 'b>(k: IndexKey, pk: &'b PrimaryKey) -> Self::StoreKey<'a, 'b>
    where
        IndexKey: 'a,
    {
        k.append(pk)
    }
}
