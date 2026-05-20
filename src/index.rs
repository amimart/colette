use std::marker::PhantomData;
use crate::store::{MultiStoreWriteHandle, ReadKVStore, WriteKVStore};
use crate::error::Error;
use crate::key::{AppendKey, HasKey, Key};

/// Index allows to maintain a separate query efficient stores on non primary-key, it is made for
/// a specific Entity and specified by a Key to index extracted from an Entity, and an IndexKind
/// (i.e. Unique or Multi).
pub trait Index<PrimaryKey: Key, Record: HasKey<PrimaryKey>> {
    type Key: Key;
    type Kind: IndexKind<Self::Key, PrimaryKey>;

    const NAME: &'static str;

    fn key(entity: &Record) -> Self::Key;

    fn set<DB: MultiStoreWriteHandle>(db: &mut DB, old: Option<(&PrimaryKey, &Record)>, new: (&PrimaryKey, &Record)) -> Result<(), Error> {
        let new_skey = Self::Kind::store_key(&Self::key(&new.1), new.0).encode();
        let old_skey = old.map(|(pk, entity)|
            Self::Kind::store_key(&Self::key(entity), pk).encode()
        );

        match old_skey {
            // Noop when the index key didn't change
            // todo: we can avoid allocations before by comparing only non encoded index keys here
            Some(old_skey) if old_skey == new_skey => {
                return Ok(())
            }
            _ => {}
        };

        let mut store = db.open_store(Self::NAME)?;

        if let Some(skey) = old_skey {
            store.remove(&skey)?;
        }

        if let Some(_) = store.get(&new_skey)? {
            Err(Error::AlreadyExists(Self::NAME.to_string()))?
        }

        // todo: we can avoid encoding the pk as value for multi indexes, as already present in the key.
        // todo: we can add a IndexKind::store_value(&PK) -> &[u8], returning the pk for unique impl and empty for multi.
        store.set(&new_skey, &new.0.encode())?;

        Ok(())
    }

    fn remove<DB: MultiStoreWriteHandle>(db: &mut DB, target: (&PrimaryKey, &Record)) -> Result<(), Error> {
        let mut store = db.open_store(Self::NAME)?;
        let ikey = Self::key(&target.1);
        let skey = Self::Kind::store_key(&ikey, target.0).encode();

        store.remove(&skey).map_err(|e| Error::Backend(e.into()))
    }
}

pub type StoreKey<'a, I, PK, T> = <<I as Index<PK, T>>::Kind as IndexKind<<I as Index<PK, T>>::Key, PK>>::StoreKey<'a>;

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

    fn store_key<'a>(k: &'a IndexKey, pk: &'a PrimaryKey) -> Self::StoreKey<'a>;
}

pub struct Unique;

impl<IndexKey, PrimaryKey> IndexKind<IndexKey, PrimaryKey> for Unique
where
    IndexKey: Key,
    PrimaryKey: Key,
{
    type StoreKey<'a> = &'a IndexKey
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: &'a IndexKey, _pk: &'a PrimaryKey) -> Self::StoreKey<'a> {
        k
    }
}

pub struct Multi;

impl<IndexKey, PrimaryKey> IndexKind<IndexKey, PrimaryKey> for Multi
where
    IndexKey: Key + AppendKey<PrimaryKey>,
    PrimaryKey: Key,
{
    type StoreKey<'a> = <IndexKey as AppendKey<PrimaryKey>>::Key<'a>
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: &'a IndexKey, pk: &'a PrimaryKey) -> Self::StoreKey<'a> {
        k.append(pk)
    }
}
