use crate::error::BackendError;
use crate::scan::{Direction, ScanRange};

pub trait MultiStore {
    type ReadHandle: MultiStoreReadHandle;
    type WriteHandle: MultiStoreWriteHandle;

    fn read(&self) -> Result<Self::ReadHandle, BackendError>;

    fn write(&self) -> Result<Self::WriteHandle, BackendError>;
}

pub trait MultiStoreReadHandle {
    type Store: ReadKVStore;

    fn open_store(&self, name: &str) -> Result<Self::Store, BackendError>;
}

pub trait MultiStoreWriteHandle {
    type Store: ReadWriteKVStore;

    fn open_store(&mut self, name: &str) -> Result<Self::Store, BackendError>;

    fn commit(self) -> Result<(), BackendError>;
}

pub trait ReadWriteKVStore: ReadKVStore + WriteKVStore {}

pub trait WriteKVStore {
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), BackendError>;

    fn remove(&mut self, key: &[u8]) -> Result<(), BackendError>;
}

pub trait ReadKVStore {
    type Iter: Iterator<Item = Result<(Vec<u8>, Vec<u8>), BackendError>>;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, BackendError>;

    fn scan(&self, range: ScanRange, direction: Direction) -> Result<Self::Iter, BackendError>;
}
