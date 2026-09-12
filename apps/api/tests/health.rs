//! Real route/OpenAPI/PostgreSQL agreement test, not a mocked database check.
//! scripts/runtime.py starts isolated Compose PostgreSQL; test config cannot use an
//! inherited production DATABASE_URL. The SELECT below inspects migration bookkeeping.

use loco_rs::testing::prelude::*;
use mobile_qa::app::App;
use mobile_qa_contracts::browser::{self, ApiError, HealthResponse, HealthStatus, HEALTH_PATH};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

#[tokio::test]
async fn registered_handler_agrees_with_openapi_and_database() {
    let spec = serde_json::to_value(browser::openapi()).unwrap();
    let paths = spec["paths"].as_object().unwrap();
    assert_eq!(paths.len(), 14);
    let operation = &paths[HEALTH_PATH]["get"];
    assert_eq!(operation["operationId"], "getHealth");
    assert!(operation.get("requestBody").is_none());
    assert!(operation.get("parameters").is_none());
    assert!(paths[HEALTH_PATH].get("post").is_none());
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HealthResponse"
    );
    assert_eq!(
        operation["responses"]["default"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ApiError"
    );

    request::<App, _, _>(|request, ctx| async move {
        let res = request.get(HEALTH_PATH).await;
        res.assert_status_ok();
        assert_eq!(res.json::<HealthResponse>(), HealthResponse {
            status: HealthStatus::Ok,
            service: "mobile-qa".into(),
            version: "0.1.0".into(),
        });
        let wrong_method = request.post(HEALTH_PATH).await;
        assert_eq!(wrong_method.status_code(), 405);
        assert_eq!(wrong_method.json::<ApiError>().code, "method_not_allowed");
        assert_eq!(wrong_method.json::<ApiError>().request_id.to_string(), wrong_method.header("x-request-id"));
        let unknown = request.get("/api/missing").await;
        unknown.assert_status_not_found();
        assert_eq!(unknown.json::<ApiError>().code, "not_found");
        let row = ctx.db.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT current_database() AS name, to_regclass('seaql_migrations')::text AS migrations")).await.unwrap().unwrap();
        assert_eq!(row.try_get::<String>("", "name").unwrap(), "mobile_qa_test");
        assert_eq!(row.try_get::<String>("", "migrations").unwrap(), "seaql_migrations");
    }).await;
}
