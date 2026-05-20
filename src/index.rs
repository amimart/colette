use crate::key::{Key};

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
