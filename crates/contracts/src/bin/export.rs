use mobile_qa_contracts::{browser, worker::WorkerContracts};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: export-contracts OUTPUT_DIRECTORY")?,
    )
    .join("contracts");
    fs::create_dir_all(&out)?;
    fs::write(
        out.join("browser.openapi.json"),
        format!("{}\n", browser::openapi().to_pretty_json()?),
    )?;
    fs::write(
        out.join("worker.schema.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&schemars::schema_for!(WorkerContracts))?
        ),
    )?;
    Ok(())
}
