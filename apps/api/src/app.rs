//! Connect this application to Loco's startup, routing and migration hooks.
//! Product rules belong in feature services, not in these framework hooks.

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
    // CLI startup uses load_config directly; auth must also exist outside the test boot hook.
    async fn load_config(environment: &Environment) -> Result<Config> {
        let mut config = environment.load()?;
        crate::config::configure_auth(environment, &mut config)?;
        Ok(config)
    }
    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        ctx.shared_store.insert(crate::config::Setup::new(&ctx)?);
        ctx.shared_store
            .insert(crate::services::execution_wakeup::ExecutionWakeup::default());
        if !matches!(ctx.environment, Environment::Test) {
            crate::services::capacity_wake::start(&ctx)
                .map_err(|e| loco_rs::Error::string(&e.message))?;
            crate::services::upload_validation::start(&ctx);
        }
        Ok(ctx)
    }
    async fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
        Ok(router.layer(axum::middleware::from_fn(
            crate::middleware::request_context,
        )))
    }
    // Device workers poll the explicit HTTP protocol; startup never launches a phone.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::health::routes())
            .add_route(controllers::setup::routes())
            .add_route(controllers::runs::routes())
            .add_route(controllers::test_library::routes())
            .add_route(controllers::worker::routes())
            .add_route(controllers::task_sessions::routes())
            .add_route(controllers::test_authoring::routes())
            .add_route(controllers::commercial::routes())
            .add_route(controllers::device_hosts::routes())
        // routes-inject (do not remove)
    }
    // No Rust background jobs are registered yet. This hook will not launch the
    // separate Python device worker; it polls our HTTP lease protocol.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::operator::Operator);
        tasks.register(crate::tasks::cleanup::Cleanup);
        tasks.register(crate::tasks::execution::Execution);
        // tasks-inject (do not remove)
    }
    // Explicit provisioning owns account creation. Startup never resets or seeds customer data.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
