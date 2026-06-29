use colette::store::MultiStore;

pub fn run_collection_contract_tests<DB: MultiStore>(_make_db: impl Fn() -> DB) {}
