use std::collections::BTreeMap;
use crate::error::BackendError;
use crate::scan::{Direction, ScanRange};
use crate::store::{MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, ReadWriteKVStore, WriteKVStore};

pub struct InMemoryMultiStore {
    stores: BTreeMap<String, InMemoryStore>,
}

impl InMemoryMultiStore {
    pub fn new() -> Self {
        Self {
            stores: BTreeMap::new(),
        }
    }
}

impl MultiStore for InMemoryMultiStore {
    type ReadHandle = InMemoryReadHandle;
    type WriteHandle = InMemoryWriteHandle;

    fn read(&self, namespace: &str) -> Result<Self::ReadHandle, BackendError> {
        todo!()
    }

    fn write(&self, namespace: &str) -> Result<Self::WriteHandle, BackendError> {
        todo!()
    }
}

pub struct InMemoryReadHandle {
}

impl MultiStoreReadHandle for InMemoryReadHandle {
    type Store = InMemoryStore;

    fn open_store(&self, name: &str) -> Result<Self::Store, BackendError> {
        todo!()
    }
}

pub struct InMemoryWriteHandle {
}

impl MultiStoreWriteHandle for InMemoryWriteHandle {
    type Store = InMemoryStore;

    fn open_store(&mut self, name: &str) -> Result<Self::Store, BackendError> {
        todo!()
    }

    fn commit(self) -> Result<(), BackendError> {
        todo!()
    }
}

pub struct InMemoryStore {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl ReadWriteKVStore for InMemoryStore {}

impl WriteKVStore for InMemoryStore {
    fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<(), BackendError> {
        todo!()
    }

    fn remove(&mut self, key: impl AsRef<[u8]>) -> Result<(), BackendError> {
        todo!()
    }
}

impl ReadKVStore for InMemoryStore {
    type Iter = std::iter::Empty<Result<(Vec<u8>, Vec<u8>), BackendError>>;

    fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>, BackendError> {
        todo!()
    }

    fn scan(&self, range: ScanRange, direction: Direction) -> Result<Self::Iter, BackendError> {
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
