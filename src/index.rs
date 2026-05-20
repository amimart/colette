use crate::error::Error;
use crate::key::{HasKey, Key};

/// Index allows to maintain a separate query efficient stores on non primary-key, it is made for
/// a specific Entity and specified by a Key to index extracted from an Entity, and an IndexKind
/// (i.e. Unique or Multi).
pub trait Index<PrimaryKey: Key, Record: HasKey<PrimaryKey>> {
    type Key: Key;
    type Kind: IndexKind<Self::Key, PrimaryKey>;

    const NAME: &'static str;

    fn key(entity: &Record) -> Self::Key;

    fn set<DB: MultiStoreWriteHandle>(db: &mut DB, old: Option<(&PrimaryKey, &Record)>, new: (&PrimaryKey, &Record)) -> Result<(), Error>;

    fn remove<DB: MultiStoreWriteHandle>(db: &mut DB, target: (&PrimaryKey, &Record)) -> Result<(), Error>;
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
