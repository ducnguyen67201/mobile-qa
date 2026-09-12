use mobile_qa_contracts::{
    browser::{ApiError, HealthResponse},
    worker::{ContractProbe, WorkerContracts},
};
use std::{env, fs, path::PathBuf};
use ts_rs::TS;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: export-contracts OUTPUT_DIRECTORY")?,
    );
    let ts = out.join("frontend/src/bindings");
    fs::create_dir_all(&ts)?;
    let config = ts_rs::Config::new().with_out_dir(&ts);
    HealthResponse::export_all(&config)?;
    ApiError::export_all(&config)?;
    ContractProbe::export_all(&config)?;
    let schema = out.join("contracts/worker.schema.json");
    fs::create_dir_all(schema.parent().ok_or("missing parent")?)?;
    fs::write(
        schema,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&schemars::schema_for!(WorkerContracts))?
        ),
    )?;
    Ok(())
}
