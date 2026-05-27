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
        Record: 'a,;

    const NAME: &'static str;

    fn key(entity: &Record) -> Self::Key<'_>;

    fn set<'a, DB: MultiStoreWriteHandle>(
        db: &mut DB,
        old: Option<(&Record::Key<'a>, &'a Record)>,
        new: (&Record::Key<'a>, &'a Record),
    ) -> Result<(), Error> {
        let new_skey = Self::Kind::store_key(Self::key(new.1), new.0);

        if let Some((pk, entity)) = old {
            let old_skey = Self::Kind::store_key(Self::key(entity), pk);

            if old_skey == new_skey {
                return Ok(());
            }

            let mut store = db.open_store(Self::NAME)?;
            store.remove(old_skey.encode())?;
        }

        let mut store = db.open_store(Self::NAME)?;

        let skey = new_skey.encode();
        if store.get(&skey)?.is_some() {
            Err(Error::AlreadyExists(Self::NAME.to_string()))?
        }

        store.set(skey, new.0.encode())?;

        Ok(())
    }

    fn remove<'a, DB: MultiStoreWriteHandle>(
        db: &mut DB,
        target: (&Record::Key<'a>, &'a Record),
    ) -> Result<(), Error> {
        let mut store = db.open_store(Self::NAME)?;
        let ikey = Self::key(target.1);
        let skey = Self::Kind::store_key(ikey, target.0);

        store.remove(skey.encode()).map_err(Error::Backend)
    }
}

pub type StoreKey<'a, I, PK, T> =
    <<I as Index<T>>::Kind<'a> as IndexKind<<I as Index<T>>::Key<'a>, PK>>::StoreKey<'a>;

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
    type StoreKey<'a>: Key
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: IndexKey, pk: &'a PrimaryKey) -> Self::StoreKey<'a>
    where
        IndexKey: 'a;
}

pub struct Unique;

impl<IndexKey, PrimaryKey> IndexKind<IndexKey, PrimaryKey> for Unique
where
    IndexKey: Key,
    PrimaryKey: Key,
{
    type StoreKey<'a>
        = IndexKey
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: IndexKey, _pk: &'a PrimaryKey) -> Self::StoreKey<'a>
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
    type StoreKey<'a>
        = <IndexKey as AppendKey<PrimaryKey>>::Key<'a>
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: IndexKey, pk: &'a PrimaryKey) -> Self::StoreKey<'a>
    where
        IndexKey: 'a,
    {
        k.append(pk)
    }
}
