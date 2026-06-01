use crate::store::MultiStore;

pub fn run_multistore_tests<DB: MultiStore>(_make_db: impl Fn() -> DB) {
    // todo: write integration tests
}
