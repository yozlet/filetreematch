use filetreematch::db::Database;
use tempfile::TempDir;

pub fn open_temp_db() -> (TempDir, Database) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    (tmp, db)
}
