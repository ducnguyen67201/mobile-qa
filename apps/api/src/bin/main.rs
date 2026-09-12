//! Loco CLI entry point. Launch from apps/api so relative config paths resolve.
//! Development launchers inject Doppler into this process; checks use isolated config.

use loco_rs::cli;
use migration::Migrator;
use mobile_qa::app::App;
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
