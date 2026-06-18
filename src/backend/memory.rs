use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use crate::error::BackendError;
use crate::scan::{Direction, ScanRange};
use crate::store::{MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, ReadWriteKVStore, WriteKVStore};

pub struct InMemoryMultiStore {
    stores: Arc<RwLock<BTreeMap<&'static str, Arc<RwLock<Arc<NamespacedStores>>>>>>,
}

pub type NamespacedStores = BTreeMap<&'static str, Arc<KVStore>>;
pub type StagedStores = BTreeMap<&'static str, KVStore>;
pub type KVStore = BTreeMap<Vec<u8>, Vec<u8>>;

impl InMemoryMultiStore {
    pub fn new() -> Self {
        Self {
            stores: Arc::from(RwLock::from(BTreeMap::new())),
        }
    }
}

impl MultiStore for InMemoryMultiStore {
    type ReadHandle = InMemoryReadHandle;
    type WriteHandle = InMemoryWriteHandle;

    fn prepare(&self, namespace: &'static str, stores: impl IntoIterator<Item=&'static str>) -> Result<(), BackendError> {
        let mut nstores = NamespacedStores::new();
        stores.into_iter().for_each(|store| {
            nstores.insert(store, Arc::new(KVStore::new()));
        });

        let mut db = self.stores.write().unwrap();
        db.insert(namespace, Arc::from(RwLock::from(Arc::from(nstores))));

        Ok(())
    }

    fn read(&self, namespace: &'static str) -> Result<Self::ReadHandle, BackendError> {
        let db = self.stores.read().unwrap();
        let nstores = db.get(namespace).unwrap();
        let snapshot = nstores.read().unwrap().clone();

        Ok(InMemoryReadHandle { stores: snapshot })
    }

    fn write(&self, namespace: &'static str) -> Result<Self::WriteHandle, BackendError> {
        let db = self.stores.read().unwrap();
        let nstores = db.get(namespace).unwrap();
        let snapshot = nstores.read().unwrap().clone();
        let staged = snapshot.iter()
            .map(|(n, s)| (*n, s.as_ref().clone()))
            .collect();

        Ok(
            InMemoryWriteHandle {
                namespace: nstores.clone(),
                staged,
            }
        )
    }
}

pub struct InMemoryReadHandle {
    stores: Arc<NamespacedStores>,
}

impl MultiStoreReadHandle for InMemoryReadHandle {
    type Store = InMemoryReadStore;

    fn open_store(&self, name: &'static str) -> Result<Self::Store, BackendError> {
        Ok(InMemoryReadStore {
            store: self.stores.get(name).unwrap().clone(),
        })
    }
}

pub struct InMemoryReadStore {
    store: Arc<KVStore>,
}

impl ReadKVStore for InMemoryReadStore {
    type Iter = std::iter::Empty<Result<(Vec<u8>, Vec<u8>), BackendError>>;

    fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>, BackendError> {
        todo!()
    }

    fn scan(self, range: ScanRange, direction: Direction) -> Result<Self::Iter, BackendError> {
        todo!()
    }
}

pub struct InMemoryWriteHandle {
    namespace: Arc<RwLock<Arc<NamespacedStores>>>,
    staged: StagedStores,
}

impl MultiStoreWriteHandle for InMemoryWriteHandle {
    type Store<'a> = InMemoryWriteStore<'a>;

    fn open_store(&mut self, name: &'static str) -> Result<Self::Store<'_>, BackendError> {
        Ok(
            InMemoryWriteStore{
                store: self.staged.get_mut(name).unwrap(),
            }
        )
    }

    fn commit(self) -> Result<(), BackendError> {
        let new_stores = self.staged.into_iter()
            .map(|(n, s)| (n, Arc::new(s)))
            .collect();

        let mut stores = self.namespace.write().unwrap();
        *stores = Arc::new(new_stores);
        Ok(())
    }
}

pub struct InMemoryWriteStore<'a> {
    store: &'a mut KVStore,
}

impl<'a> ReadWriteKVStore<'a> for InMemoryWriteStore<'a> {}

impl<'a> WriteKVStore<'a> for InMemoryWriteStore<'a> {
    fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<(), BackendError> {
        todo!()
    }

    fn remove(&mut self, key: impl AsRef<[u8]>) -> Result<(), BackendError> {
        todo!()
    }
}

impl ReadKVStore for InMemoryWriteStore<'_> {
    type Iter = std::iter::Empty<Result<(Vec<u8>, Vec<u8>), BackendError>>;

    fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>, BackendError> {
        todo!()
    }

    fn scan(self, range: ScanRange, direction: Direction) -> Result<Self::Iter, BackendError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::tests::run_multistore_tests;

    #[test]
    fn contract() {
        run_multistore_tests(|| InMemoryMultiStore::new());
    }
}
