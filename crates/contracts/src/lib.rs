//! Transport source of truth shared by the API, browser and fixture worker.
//! Keep this crate independent of Loco, SeaORM and device SDKs so code generation
//! needs neither a running server nor a database. See docs/architect/contracts.md.

pub mod browser;
pub mod worker;

pub mod execution;
pub mod execution_api;

pub mod test_library;
pub mod test_library_api;

pub mod task_sessions;
pub mod task_sessions_api;

pub mod automation;
pub mod automation_api;

pub mod execution_lifecycle;

pub mod regression;
pub mod regression_api;
