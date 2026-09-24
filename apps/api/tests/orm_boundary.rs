//! Architectural boundary: backend persistence stays on SeaORM/SeaQuery's typed APIs.
use std::{fs, path::Path};

fn rust_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("read ORM boundary root") {
        let path = entry.expect("read ORM boundary entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path
                .file_name()
                .is_none_or(|name| name != "orm_boundary.rs")
        {
            files.push(path);
        }
    }
}

#[test]
fn backend_has_no_raw_sql_escape_hatches() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest.join("src"),
        manifest.join("tests"),
        manifest.join("migration/src"),
    ];
    let forbidden_apis = [
        ["execute", "_unprepared"].concat(),
        ["query_all", "_raw"].concat(),
        ["query_one", "_raw"].concat(),
        ["execute", "_raw"].concat(),
        ["from_sql", "_and_values"].concat(),
        ["Statement::from", "_string"].concat(),
        ["Expr::", "cust"].concat(),
    ];
    let sql_starters = [
        "SELECT ",
        "INSERT ",
        "UPDATE ",
        "DELETE ",
        "CREATE TABLE ",
        "ALTER TABLE ",
        "DROP TABLE ",
        "WITH ",
    ];
    let mut violations = Vec::new();

    for root in roots {
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        for path in files {
            let source = fs::read_to_string(&path).expect("read Rust source");
            for api in &forbidden_apis {
                if source.contains(api) {
                    violations.push(format!("{} uses {api}", path.display()));
                }
            }
            for (index, line) in source.lines().enumerate() {
                if sql_starters.iter().any(|starter| {
                    line.contains(&format!("\"{starter}"))
                        || line.contains(&format!("#\"{starter}"))
                }) {
                    violations.push(format!(
                        "{}:{} contains a SQL-leading string literal",
                        path.display(),
                        index + 1
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "raw SQL is outside the backend persistence boundary:\n{}",
        violations.join("\n")
    );
}
