use std::marker::PhantomData;
use crate::error::Error;
use crate::index::{Index, IndexKind};
use crate::key::{HasKey, Key};
use crate::store::MultiStoreWriteHandle;

// HList helper types:
pub struct Nil;
pub struct Here;
pub struct There<Tail>(PhantomData<Tail>);
pub struct Cons<Head, Tail>(PhantomData<(Head, Tail)>);

/// IndexRegistry is a recursive HList trait to allow defining multiple indexes as generic types.
pub trait IndexRegistry<PK: Key, T: HasKey<PK>> {
    fn set<DB: MultiStoreWriteHandle>(
        db: &mut DB,
        old: Option<(&PK, &T)>,
        new: (&PK, &T),
    ) -> Result<(), Error>;

    fn remove<DB: MultiStoreWriteHandle>(db: &mut DB, target: (&PK, &T)) -> Result<(), Error>;

    fn has_index(name: &str) -> bool;
}

impl<PK: Key, T: HasKey<PK>> IndexRegistry<PK, T> for Nil {
    fn set<DB: MultiStoreWriteHandle>(
        _db: &mut DB,
        _old: Option<(&PK, &T)>,
        _new: (&PK, &T),
    ) -> Result<(), Error> {
        Ok(())
    }

    fn remove<DB: MultiStoreWriteHandle>(_db: &mut DB, _target: (&PK, &T)) -> Result<(), Error> {
        Ok(())
    }

    fn has_index(_name: &str) -> bool {
        false
    }
}

impl<PK, T, Head, Tail> IndexRegistry<PK, T> for Cons<Head, Tail>
where
    PK: Key,
    T: HasKey<PK>,
    Head: Index<PK, T>,
    Head::Kind: IndexKind<Head::Key, PK>,
    Tail: IndexRegistry<PK, T>,
{
    fn set<DB: MultiStoreWriteHandle>(
        db: &mut DB,
        old: Option<(&PK, &T)>,
        new: (&PK, &T),
    ) -> Result<(), Error> {
        Head::set(db, old, new)?;
        Tail::set(db, old, new)
    }

    fn remove<DB: MultiStoreWriteHandle>(db: &mut DB, target: (&PK, &T)) -> Result<(), Error> {
        Head::remove(db, target)?;
        Tail::remove(db, target)
    }

    fn has_index(name: &str) -> bool {
        Head::NAME == name || Tail::has_index(name)
    }
}

/// ContainsIndex is used check the presence of an index in the registry HList.
pub trait ContainsIndex<I, Proof> {}

impl<I, Tail> ContainsIndex<I, Here> for Cons<I, Tail> {}

impl<I, Head, Tail, Proof> ContainsIndex<I, There<Proof>> for Cons<Head, Tail> where
    Tail: ContainsIndex<I, Proof>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{BackendError, Error};
    use crate::index::Unique;
    use crate::key::HasKey;
    use crate::scan::{Direction, ScanRange};
    use crate::store::{MultiStoreWriteHandle, ReadKVStore, ReadWriteKVStore, WriteKVStore};

    // ── Minimal entity ────────────────────────────────────────────────────────

    #[derive(Clone)]
    struct Record(u32);

    impl HasKey<u32> for Record {
        fn key(&self) -> u32 {
            self.0
        }
    }

    // ── Mock indexes ──────────────────────────────────────────────────────────
    // Override set/remove to record invocation via open_store, bypassing the
    // default implementation so the test stays focused on IndexRegistry dispatch.

    struct IndexA;
    struct IndexB;
    struct FailIndex;

    macro_rules! spy_index {
        ($ty:ty, $name:literal) => {
            impl Index<u32, Record> for $ty {
                type Key = u32;
                type Kind = Unique;
                const NAME: &'static str = $name;
                fn key(r: &Record) -> u32 {
                    r.0
                }
                fn set<DB: MultiStoreWriteHandle>(
                    db: &mut DB,
                    _old: Option<(&u32, &Record)>,
                    _new: (&u32, &Record),
                ) -> Result<(), Error> {
                    db.open_store(Self::NAME)?;
                    Ok(())
                }
                fn remove<DB: MultiStoreWriteHandle>(
                    db: &mut DB,
                    _target: (&u32, &Record),
                ) -> Result<(), Error> {
                    db.open_store(Self::NAME)?;
                    Ok(())
                }
            }
        };
    }

    spy_index!(IndexA, "index_a");
    spy_index!(IndexB, "index_b");

    impl Index<u32, Record> for FailIndex {
        type Key = u32;
        type Kind = Unique;
        const NAME: &'static str = "fail";
        fn key(r: &Record) -> u32 {
            r.0
        }
        fn set<DB: MultiStoreWriteHandle>(
            _db: &mut DB,
            _old: Option<(&u32, &Record)>,
            _new: (&u32, &Record),
        ) -> Result<(), Error> {
            Err(Error::Unexpected("injected".into()))
        }
        fn remove<DB: MultiStoreWriteHandle>(
            _db: &mut DB,
            _target: (&u32, &Record),
        ) -> Result<(), Error> {
            Err(Error::Unexpected("injected".into()))
        }
    }

    // ── Spy write handle ──────────────────────────────────────────────────────
    // Records which store names were opened; that is how we observe index dispatch.

    struct NoopStore;

    impl ReadKVStore for NoopStore {
        type Iter = std::iter::Empty<Result<(Vec<u8>, Vec<u8>), BackendError>>;
        fn get(&self, _: &[u8]) -> Result<Option<Vec<u8>>, BackendError> {
            Ok(None)
        }
        fn scan(&self, _: ScanRange, _: Direction) -> Result<Self::Iter, BackendError> {
            Ok(std::iter::empty())
        }
    }
    impl WriteKVStore for NoopStore {
        fn set(&mut self, _: &[u8], _: &[u8]) -> Result<(), BackendError> {
            Ok(())
        }
        fn remove(&mut self, _: &[u8]) -> Result<(), BackendError> {
            Ok(())
        }
    }
    impl ReadWriteKVStore for NoopStore {}

    struct Spy(Vec<String>);

    impl Spy {
        fn new() -> Self {
            Self(Vec::new())
        }
        fn invoked(&self) -> Vec<&str> {
            self.0.iter().map(String::as_str).collect()
        }
    }

    impl MultiStoreWriteHandle for Spy {
        type Store = NoopStore;
        fn open_store(&mut self, name: &str) -> Result<NoopStore, BackendError> {
            self.0.push(name.to_string());
            Ok(NoopStore)
        }
        fn commit(self) -> Result<(), BackendError> {
            Ok(())
        }
    }

    // ── has_index ─────────────────────────────────────────────────────────────

    #[test]
    fn has_index() {
        let cases: &[(fn(&str) -> bool, &str, bool)] = &[
            (<Nil as IndexRegistry<u32, Record>>::has_index, "index_a", false),
            (<Nil as IndexRegistry<u32, Record>>::has_index, "", false),
            (<Cons<IndexA, Nil> as IndexRegistry<u32, Record>>::has_index, "index_a", true),
            (<Cons<IndexA, Nil> as IndexRegistry<u32, Record>>::has_index, "index_b", false),
            (<Cons<IndexA, Cons<IndexB, Nil>> as IndexRegistry<u32, Record>>::has_index, "index_a", true),
            (<Cons<IndexA, Cons<IndexB, Nil>> as IndexRegistry<u32, Record>>::has_index, "index_b", true),
            (<Cons<IndexA, Cons<IndexB, Nil>> as IndexRegistry<u32, Record>>::has_index, "nonexistent", false),
        ];

        for &(has_index, name, expected) in cases {
            assert_eq!(has_index(name), expected, "has_index({name:?}) should be {expected}");
        }
    }

    // ── set ───────────────────────────────────────────────────────────────────

    #[test]
    fn set() {
        let record = Record(1);
        let pk = 1u32;

        let cases: &[(&dyn Fn(&mut Spy) -> Result<(), Error>, &[&str], bool)] = &[
            (
                &|s| <Nil as IndexRegistry<u32, Record>>::set(s, None, (&pk, &record)),
                &[],
                false,
            ),
            (
                &|s| <Cons<IndexA, Cons<IndexB, Nil>> as IndexRegistry<u32, Record>>::set(s, None, (&pk, &record)),
                &["index_a", "index_b"],
                false,
            ),
            (
                &|s| <Cons<FailIndex, Cons<IndexA, Nil>> as IndexRegistry<u32, Record>>::set(s, None, (&pk, &record)),
                &[],
                true,
            ),
        ];

        for (invoke, expected_invoked, expect_err) in cases {
            let mut spy = Spy::new();
            let result = invoke(&mut spy);
            assert_eq!(result.is_err(), *expect_err);
            assert_eq!(&spy.invoked(), expected_invoked);
        }
    }

    // ── remove ────────────────────────────────────────────────────────────────

    #[test]
    fn remove() {
        let record = Record(1);
        let pk = 1u32;

        let cases: &[(&dyn Fn(&mut Spy) -> Result<(), Error>, &[&str], bool)] = &[
            (
                &|s| <Nil as IndexRegistry<u32, Record>>::remove(s, (&pk, &record)),
                &[],
                false,
            ),
            (
                &|s| <Cons<IndexA, Cons<IndexB, Nil>> as IndexRegistry<u32, Record>>::remove(s, (&pk, &record)),
                &["index_a", "index_b"],
                false,
            ),
            (
                &|s| <Cons<FailIndex, Cons<IndexA, Nil>> as IndexRegistry<u32, Record>>::remove(s, (&pk, &record)),
                &[],
                true,
            ),
        ];

        for (invoke, expected_invoked, expect_err) in cases {
            let mut spy = Spy::new();
            let result = invoke(&mut spy);
            assert_eq!(result.is_err(), *expect_err);
            assert_eq!(&spy.invoked(), expected_invoked);
        }
    }

    // ── ContainsIndex ─────────────────────────────────────────────────────────

    fn assert_contains<R, I, P>()
    where
        R: ContainsIndex<I, P>,
    {
    }

    #[test]
    fn contains_index_at_head() {
        assert_contains::<Cons<IndexA, Nil>, IndexA, Here>();
    }

    #[test]
    fn contains_index_in_tail() {
        assert_contains::<Cons<IndexA, Cons<IndexB, Nil>>, IndexB, There<Here>>();
    }

    #[test]
    fn contains_index_tracks_depth() {
        struct IndexC;
        spy_index!(IndexC, "index_c");
        type R = Cons<IndexA, Cons<IndexB, Cons<IndexC, Nil>>>;
        assert_contains::<R, IndexA, Here>();
        assert_contains::<R, IndexB, There<Here>>();
        assert_contains::<R, IndexC, There<There<Here>>>();
    }
}