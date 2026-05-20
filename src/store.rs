use crate::error::BackendError;

pub trait ReadWriteKVStore: ReadKVStore + WriteKVStore {}

pub trait WriteKVStore {
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), BackendError>;

    fn remove(&mut self, key: &[u8]) -> Result<(), BackendError>;
}

pub trait ReadKVStore {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, BackendError>;
}