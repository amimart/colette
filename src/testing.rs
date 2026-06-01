//! Reusable mock implementations of the [`crate::store`] traits for use across
//! test modules.
//!
//! Every operation performed through a [`MockDb`] is recorded in a shared
//! [`TxLog`] that callers can inspect after the fact.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::BackendError;
use crate::scan::{Direction, ScanRange};
use crate::store::{
    MultiStore, MultiStoreReadHandle, MultiStoreWriteHandle, ReadKVStore, ReadWriteKVStore,
    WriteKVStore,
};

// ── Error helpers ─────────────────────────────────────────────────────────────

/// Constructs a [`BackendError`] from a static message, for use in error
/// factory functions passed to [`MockDb::with_write_err`] /
/// [`MockDb::with_commit_err`].
pub fn backend_error(msg: &'static str) -> BackendError {
    BackendError::new(std::io::Error::new(std::io::ErrorKind::Other, msg))
}

// ── TxLog ─────────────────────────────────────────────────────────────────────

/// Shared log of all store operations performed via a [`MockDb`] instance.
///
/// The log is shared between the db and all handles/stores it produces so
/// that callers can observe the full sequence of operations after a test call.
#[derive(Default, Debug)]
pub struct TxLog {
    /// Names of stores opened via `open_store` (in call order).
    pub opens: Vec<String>,
    /// Raw keys passed to `get` (in call order).
    pub gets: Vec<Vec<u8>>,
    /// `(key, value)` pairs passed to `set` (in call order).
    pub sets: Vec<(Vec<u8>, Vec<u8>)>,
    /// Raw keys passed to `remove` (in call order).
    pub removes: Vec<Vec<u8>>,
    /// Whether `commit` was called on the write handle.
    pub committed: bool,
}

// ── MockStore ─────────────────────────────────────────────────────────────────

/// A mock [`ReadWriteKVStore`] that records every operation in a shared
/// [`TxLog`] and serves pre-configured byte values for `get` calls.
pub struct MockStore {
    log: Rc<RefCell<TxLog>>,
    data: HashMap<Vec<u8>, Vec<u8>>,
}

impl ReadKVStore for MockStore {
    type Iter = std::iter::Empty<Result<(Vec<u8>, Vec<u8>), BackendError>>;

    fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>, BackendError> {
        let key = key.as_ref().to_vec();
        self.log.borrow_mut().gets.push(key.clone());
        Ok(self.data.get(&key).cloned())
    }

    fn scan(&self, _: ScanRange, _: Direction) -> Result<Self::Iter, BackendError> {
        Ok(std::iter::empty())
    }
}

impl WriteKVStore for MockStore {
    fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<(), BackendError> {
        self.log
            .borrow_mut()
            .sets
            .push((key.as_ref().to_vec(), value.as_ref().to_vec()));
        Ok(())
    }

    fn remove(&mut self, key: impl AsRef<[u8]>) -> Result<(), BackendError> {
        self.log.borrow_mut().removes.push(key.as_ref().to_vec());
        Ok(())
    }
}

impl ReadWriteKVStore for MockStore {}

// ── MockWriteHandle ───────────────────────────────────────────────────────────

/// A mock [`MultiStoreWriteHandle`].
///
/// Each call to `open_store` is logged and returns a [`MockStore`] seeded with
/// the data registered under that store name on the owning [`MockDb`].
pub struct MockWriteHandle {
    log: Rc<RefCell<TxLog>>,
    store_data: HashMap<String, HashMap<Vec<u8>, Vec<u8>>>,
    commit_err: Option<fn() -> BackendError>,
}

impl MultiStoreWriteHandle for MockWriteHandle {
    type Store = MockStore;

    fn open_store(&mut self, name: &str) -> Result<MockStore, BackendError> {
        self.log.borrow_mut().opens.push(name.to_string());
        let data = self.store_data.get(name).cloned().unwrap_or_default();
        Ok(MockStore {
            log: self.log.clone(),
            data,
        })
    }

    fn commit(self) -> Result<(), BackendError> {
        self.log.borrow_mut().committed = true;
        match self.commit_err {
            Some(make_err) => Err(make_err()),
            None => Ok(()),
        }
    }
}

// ── MockReadHandle ────────────────────────────────────────────────────────────

/// A mock [`MultiStoreReadHandle`].
///
/// Read operations are also recorded in the shared log.
pub struct MockReadHandle {
    log: Rc<RefCell<TxLog>>,
    store_data: HashMap<String, HashMap<Vec<u8>, Vec<u8>>>,
}

impl MultiStoreReadHandle for MockReadHandle {
    type Store = MockStore;

    fn open_store(&self, name: &str) -> Result<MockStore, BackendError> {
        self.log.borrow_mut().opens.push(name.to_string());
        let data = self.store_data.get(name).cloned().unwrap_or_default();
        Ok(MockStore {
            log: self.log.clone(),
            data,
        })
    }
}

// ── MockDb ────────────────────────────────────────────────────────────────────

/// A configurable mock [`MultiStore`].
///
/// # Usage
///
/// ```rust,ignore
/// let db = MockDb::new()
///     .with_data("__main", enc_pk, enc_val)   // simulate existing record
///     .with_commit_err(|| backend_error("disk full"));
///
/// let log = db.log();  // clone the Rc before the db is moved
/// collection.insert(record)?;
///
/// let log = log.borrow();
/// assert_eq!(log.sets.len(), 1);
/// assert!(log.committed);
/// ```
pub struct MockDb {
    log: Rc<RefCell<TxLog>>,
    store_data: HashMap<String, HashMap<Vec<u8>, Vec<u8>>>,
    read_err: Option<fn() -> BackendError>,
    write_err: Option<fn() -> BackendError>,
    commit_err: Option<fn() -> BackendError>,
}

impl Default for MockDb {
    fn default() -> Self {
        Self {
            log: Rc::new(RefCell::new(TxLog::default())),
            store_data: HashMap::new(),
            read_err: None,
            write_err: None,
            commit_err: None,
        }
    }
}

impl MockDb {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a handle to the shared operation log for post-call assertions.
    ///
    /// Clone this before moving `MockDb` into a [`Collection`].
    pub fn log(&self) -> Rc<RefCell<TxLog>> {
        self.log.clone()
    }

    /// Pre-seeds a named store with a key/value entry (returned by `get`).
    pub fn with_data(
        mut self,
        store: &str,
        key: impl Into<Vec<u8>>,
        value: impl Into<Vec<u8>>,
    ) -> Self {
        self.store_data
            .entry(store.to_string())
            .or_default()
            .insert(key.into(), value.into());
        self
    }

    /// Makes `read()` return an error produced by `make_err`.
    pub fn with_read_err(mut self, make_err: fn() -> BackendError) -> Self {
        self.read_err = Some(make_err);
        self
    }

    /// Makes `write()` return an error produced by `make_err`.
    pub fn with_write_err(mut self, make_err: fn() -> BackendError) -> Self {
        self.write_err = Some(make_err);
        self
    }

    /// Makes `commit()` return an error produced by `make_err`.
    pub fn with_commit_err(mut self, make_err: fn() -> BackendError) -> Self {
        self.commit_err = Some(make_err);
        self
    }
}

impl MultiStore for MockDb {
    type ReadHandle = MockReadHandle;
    type WriteHandle = MockWriteHandle;

    fn read(&self, _: &str) -> Result<MockReadHandle, BackendError> {
        if let Some(make_err) = self.read_err {
            return Err(make_err());
        }
        Ok(MockReadHandle {
            log: self.log.clone(),
            store_data: self.store_data.clone(),
        })
    }

    fn write(&self, _: &str) -> Result<MockWriteHandle, BackendError> {
        if let Some(make_err) = self.write_err {
            return Err(make_err());
        }
        Ok(MockWriteHandle {
            log: self.log.clone(),
            store_data: self.store_data.clone(),
            commit_err: self.commit_err,
        })
    }
}
