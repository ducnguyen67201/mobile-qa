//! Transport source of truth shared by the API, browser and fixture worker.
//! Keep this crate independent of Loco, SeaORM and device SDKs so code generation
//! needs neither a running server nor a database. See docs/architect/contracts.md.

pub mod browser;
pub mod worker;
