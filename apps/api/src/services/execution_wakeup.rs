//! Wake local long-poll requests only after durable queue changes commit.
//! A periodic database recheck also discovers work committed by another API process.
use loco_rs::app::AppContext;
use tokio::sync::watch;

#[derive(Clone)]
pub struct ExecutionWakeup(watch::Sender<u64>);

impl Default for ExecutionWakeup {
    fn default() -> Self {
        Self(watch::channel(0).0)
    }
}

pub fn subscribe(ctx: &AppContext) -> watch::Receiver<u64> {
    ctx.shared_store
        .get::<ExecutionWakeup>()
        .expect("execution wakeup initialized")
        .0
        .subscribe()
}

pub fn notify(ctx: &AppContext) {
    ctx.shared_store
        .get::<ExecutionWakeup>()
        .expect("execution wakeup initialized")
        .0
        .send_modify(|version| *version = version.wrapping_add(1));
}
