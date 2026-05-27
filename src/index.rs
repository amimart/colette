use crate::entity::Entity;
use crate::error::Error;
use crate::key::{AppendKey, Key};
use crate::store::{MultiStoreWriteHandle, ReadKVStore, WriteKVStore};

/// Index allows to maintain a separate query efficient stores on non primary-key, it is made for
/// a specific Entity and specified by a Key to index extracted from an Entity, and an IndexKind
/// (i.e. Unique or Multi).
pub trait Index<'a, Record: Entity>
where
    Record: 'a,
{
    type Key: Key;
    type Kind: IndexKind<Self::Key, Record::Key<'a>>;

    const NAME: &'static str;

    fn key(entity: &Record) -> Self::Key;

    fn set<DB: MultiStoreWriteHandle>(
        db: &mut DB,
        old: Option<(&Record::Key<'a>, &Record)>,
        new: (&Record::Key<'a>, &Record),
    ) -> Result<(), Error> {
        let new_ikey = Self::key(new.1);
        let new_skey = Self::Kind::store_key(&new_ikey, new.0);

        let mut store: Option<DB::Store> = None;
        match old {
            Some((pk, entity)) => {
                let old_ikey = Self::key(entity);
                let old_skey = Self::Kind::store_key(&old_ikey, pk);

                if old_skey == new_skey {
                    return Ok(());
                }

                store.get_or_insert(db.open_store(Self::NAME)?)
                    .remove(old_skey.encode())?;
            },
            None => {}
        };

        let mut store = store.unwrap_or(db.open_store(Self::NAME)?);

        let skey = new_skey.encode();
        if store.get(&skey)?.is_some() {
            Err(Error::AlreadyExists(Self::NAME.to_string()))?
        }

        store.set(skey, &new.0.encode())?;

        Ok(())
    }

    fn remove<DB: MultiStoreWriteHandle>(
        db: &mut DB,
        target: (&Record::Key<'a>, &Record),
    ) -> Result<(), Error> {
        let mut store = db.open_store(Self::NAME)?;
        let ikey = Self::key(target.1);
        let skey = Self::Kind::store_key(&ikey, target.0);

        store.remove(skey.encode()).map_err(Error::Backend)
    }
}

pub type StoreKey<'a, I, PK, T> =
    <<I as Index<'a, T>>::Kind as IndexKind<<I as Index<'a, T>>::Key, PK>>::StoreKey<'a>;

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
    type StoreKey<'a>
        = &'a IndexKey
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
    type StoreKey<'a>
        = <IndexKey as AppendKey<PrimaryKey>>::Key<'a>
    where
        IndexKey: 'a,
        PrimaryKey: 'a;

    fn store_key<'a>(k: &'a IndexKey, pk: &'a PrimaryKey) -> Self::StoreKey<'a> {
        k.append(pk)
    }
}
