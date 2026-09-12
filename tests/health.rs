use loco_rs::testing::prelude::*;
use mobile_qa::app::App;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
#[tokio::test]
async fn health_and_isolated_database_work() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request.get("/api/health").await;
        res.assert_status_ok();
        res.assert_json(&serde_json::json!({"status":"ok","service":"mobile-qa","version":"0.1.0"}));
        request.get("/api/missing").await.assert_status_not_found();
        let row = ctx.db.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT current_database() AS name, to_regclass('seaql_migrations')::text AS migrations")).await.unwrap().unwrap();
        assert_eq!(row.try_get::<String>("", "name").unwrap(), "mobile_qa_test");
        assert_eq!(row.try_get::<String>("", "migrations").unwrap(), "seaql_migrations");
    }).await;
}
