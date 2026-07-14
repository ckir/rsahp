//! Proves the percent-encoded absolute SQLite URL from `common::datadir` actually
//! connects + migrates via sea-orm on the host OS, including a path with a SPACE.

use common::datadir::database_url_from_path;

#[tokio::test]
async fn sea_orm_connects_and_migrates_via_encoded_absolute_url_with_space() {
    // Deliberately include a space to mimic `C:\Users\John Doe\...`.
    let dir = std::env::temp_dir()
        .join("rsahp url test with space")
        .join(format!("run_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("rsahp.db");

    let url = database_url_from_path(&db_path);
    assert!(
        url.contains("%20"),
        "spaced path must be percent-encoded: {url}"
    );

    let db = sea_orm::Database::connect(&url)
        .await
        .expect("sea-orm must connect to the encoded absolute sqlite URL");
    backend::setup_schema(&db)
        .await
        .expect("migrations must run against the resolved URL");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
