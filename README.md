# Colette

![Status](https://img.shields.io/badge/status-🚧%20WIP-yellow?style=for-the-badge)
[![lint](https://img.shields.io/github/actions/workflow/status/amimart/colette/lint.yaml?label=lint&style=for-the-badge&logo=github)](https://github.com/amimart/colette/actions/workflows/lint.yaml)
[![build](https://img.shields.io/github/actions/workflow/status/amimart/colette/build.yaml?label=build&style=for-the-badge&logo=github)](https://github.com/amimart/colette/actions/workflows/build.yaml)
[![test](https://img.shields.io/github/actions/workflow/status/amimart/colette/test.yaml?label=test&style=for-the-badge&logo=github)](https://github.com/amimart/colette/actions/workflows/test.yaml)

Colette - Typed collections, indexes and scans over KV stores

## Purpose

Colette aims to fill the gap between low-level embedded key-value stores and heavier SQL/ORM-based solutions.

Colette is designed as a lightweight typed storage layer on top of ordered KV stores, focused on:

* typed records;
* indexes;
* prefix scans;
* cursor-based pagination;
* range queries;
* multi-backend support;

The goal is not to build:

* an ORM;
* a SQL engine;
* a query planner;
* or a database server.

## Status

🚧 Under active Design, you'll find below some first sketches.

### Index definition

```rust
pub struct ByStatusAndSize;
impl Index<InfoHash, Download> for ByStatusAndSize {
    type Key = (Status, u64);
    type Kind = Multi;
    const NAME: &'static str = "status_size";

    fn key(entity: &Download) -> Self::Key {
        (entity.status, entity.size)
    }
}
```

### Collection definition

```rust
let downloads = Collection::builder::<Vec<u8>, Download>("downloads", DB {})
    .with_index::<UniqueName>()
    .with_index::<ByStatus>()
    .with_index::<ByStatusAndSize>()
    build();

downloads.save(dl)?;
let my_dl = downloads.get(&dl.info_hash)?;
```

### Index scans

```rust
downloads.index(ByStatusAndSize).?
    .prefix_range(
        Bound::Included((Status::InProgress, 0))..Bound::Excluded((Status::InProgress, 1000000))
    ).direction(Direction::LeftToRight)
    .after(cursor)
    .iter();
```


