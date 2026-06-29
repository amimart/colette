mod common;

use colette::backend::memory::InMemoryMultiStore;

#[test]
fn memory_collection_contract() {
    common::run_collection_contract_tests(InMemoryMultiStore::new);
}
