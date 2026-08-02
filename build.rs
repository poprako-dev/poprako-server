use std::path::{Path, PathBuf};
use std::{env, fs, io};

fn migration_up_paths(migrations_dir: &Path) -> io::Result<Vec<PathBuf>> {
    //
    let mut migration_up_paths = Vec::new();

    for entry_rest in fs::read_dir(migrations_dir)? {
        //
        let entry = entry_rest?;

        if !entry.file_type()?.is_dir() {
            continue;
        }

        let migration_up_path = entry.path().join("up.sql");

        if !migration_up_path.is_file() {
            continue;
        }

        migration_up_paths.push(migration_up_path);
    }

    migration_up_paths.sort();

    Ok(migration_up_paths)
}

fn prepare_sql(migration_up_paths: &[PathBuf]) -> io::Result<String> {
    //
    let mut prepare_sql = String::new();

    for migration_up_path in migration_up_paths {
        //
        let migration_sql = fs::read_to_string(migration_up_path)?;

        prepare_sql.push_str(&migration_sql);

        prepare_sql.push('\n');
    }

    Ok(prepare_sql)
}

fn main() -> io::Result<()> {
    //
    println!("cargo:rerun-if-changed=migrations");

    let migrations_dir = Path::new("migrations");

    let migration_up_paths = migration_up_paths(migrations_dir)?;

    if migration_up_paths.is_empty() {
        return Err(io::Error::other("no migration up.sql files found"));
    }

    let prepare_sql = prepare_sql(&migration_up_paths)?;

    let output_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("OUT_DIR is not set"))?;

    let output_path = output_dir.join("prepare.sql");

    fs::write(output_path, prepare_sql)
}
