use crate::error::BackendError;

pub trait MultiStore {
    type ReadHandle<'a>: MultiStoreReadHandle
    where
        Self: 'a;
    type WriteHandle<'a>: MultiStoreWriteHandle
    where
        Self: 'a;

    fn read(&self) -> Result<Self::ReadHandle<'_>, BackendError>;

    fn write(&self) -> Result<Self::WriteHandle<'_>, BackendError>;
}

pub trait MultiStoreReadHandle {
    type Store<'a>: ReadKVStore
    where
        Self: 'a;

    fn open_store(&self, name: &str) -> Result<Self::Store<'_>, BackendError>;
}

pub trait MultiStoreWriteHandle {
    type Store<'a>: ReadWriteKVStore
    where
        Self: 'a;

    fn open_store(&mut self, name: &str) -> Result<Self::Store<'_>, BackendError>;

    fn commit(self) -> Result<(), BackendError>;
}

pub trait ReadWriteKVStore: ReadKVStore + WriteKVStore {}

pub trait WriteKVStore {
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), BackendError>;

    fn remove(&mut self, key: &[u8]) -> Result<(), BackendError>;
}

pub trait ReadKVStore {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, BackendError>;
}
