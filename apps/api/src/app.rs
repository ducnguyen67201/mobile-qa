//! Connect this application to Loco's startup, routing and migration hooks.
//! Product rules belong in future feature services, not in these framework hooks.

// Adapted from Loco v1.1.0 base_template (Apache-2.0); see NOTICE.
use crate::controllers;
use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        "mobile_qa"
    }
    fn app_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }
    // No custom integrations need initialization in the foundation.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes().add_route(controllers::health::routes())
        // routes-inject (do not remove)
    }
    // No Rust background jobs are registered yet. This hook will not launch the
    // separate Python device worker; that worker needs the planned HTTP protocol.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }
    fn register_tasks(_tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    // Required framework hooks remain no-ops: there are no product tables or seed
    // records yet. Do not add destructive resets to normal application startup.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
