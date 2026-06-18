use crate::error::BackendError;
use crate::scan::{Direction, ScanRange};

pub trait MultiStore {
    type ReadHandle: MultiStoreReadHandle;
    type WriteHandle: MultiStoreWriteHandle;

    /// Initializes the given stores for this namespace, if not existing already. Using a non-initialized
    /// namespace or store will panic.
    fn prepare(&self, namespace: &'static str, stores: impl IntoIterator<Item = &'static str>) -> Result<(), BackendError>;

    fn read(&self, namespace: &'static str) -> Result<Self::ReadHandle, BackendError>;

    /// Opens a MultiStoreWriteHandle for the given namespace. All writes to stores opened from this
    /// handle will be atomic when commit() is called.
    fn write(&self, namespace: &'static str) -> Result<Self::WriteHandle, BackendError>;
}

pub trait MultiStoreReadHandle {
    type Store: ReadKVStore;

    fn open_store(&self, name: &'static str) -> Result<Self::Store, BackendError>;
}

/// A MultiStoreWriteHandle provides atomic writes across all stores opened from it.
pub trait MultiStoreWriteHandle {
    type Store<'a>: ReadWriteKVStore<'a>
    where
        Self: 'a;

    fn open_store(&mut self, name: &'static str) -> Result<Self::Store<'_>, BackendError>;

    fn commit(self) -> Result<(), BackendError>;
}

pub trait ReadWriteKVStore<'a>: ReadKVStore + WriteKVStore<'a> {}

pub trait WriteKVStore<'a> {
    fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<(), BackendError>;

    fn remove(&mut self, key: impl AsRef<[u8]>) -> Result<(), BackendError>;
}

pub trait ReadKVStore {
    type Iter<'a>: Iterator<Item = Result<(Vec<u8>, Vec<u8>), BackendError>>
    where
        Self: 'a;

    fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>, BackendError>;

    fn scan(&self, range: ScanRange, direction: Direction) -> Result<Self::Iter<'_>, BackendError>;
}
