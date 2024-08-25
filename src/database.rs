use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite};
use tokio::sync::OnceCell;

use crate::config::CONFIG;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

pub async fn init() {
    let database_url = &CONFIG.database_url;

    if !Sqlite::database_exists(database_url)
        .await
        .expect("Failed to check database exists.")
    {
        Sqlite::create_database(database_url)
            .await
            .expect("Falied to create database.");
    }

    let pool = SqlitePool::connect(database_url)
        .await
        .expect("Failed connect to database.");

    sqlx::migrate!("db/migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate database.");

    POOL.set(pool).expect("Failed to set connection pool");
}

pub async fn get_pool() -> SqlitePool {
    POOL.get().expect("Failed to get connection pool.").clone()
}
