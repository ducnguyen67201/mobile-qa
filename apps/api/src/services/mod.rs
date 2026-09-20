//! Scoped application services. No device execution lives in this module.
pub mod apk_validation;
pub mod apps;
pub mod auth;
pub mod uploads;

pub mod google;

pub mod workspaces;

pub mod execution_store;

pub mod test_definitions;

pub mod run_comparison;
pub mod run_history;
pub mod runs;

pub mod scheduler;

pub mod worker_auth;

pub mod run_artifacts;

pub mod verification;

pub mod execution_wakeup;

pub mod test_library;
pub mod test_library_mutations;

pub mod task_sessions;

pub mod test_authoring;

pub mod authoring_validation;

pub mod execution_preflight;
pub mod execution_readiness;

pub mod test_library_save;
