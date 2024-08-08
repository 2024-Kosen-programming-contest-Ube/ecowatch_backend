use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Pool, Sqlite};
use std::env;
use tokio::sync::OnceCell;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

pub async fn init() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    if Sqlite::database_exists(&database_url)
        .await
        .expect("Failed to check database exists.")
    {
        Sqlite::create_database(&database_url)
            .await
            .expect("Falied to create database.")
    }

    let pool = SqlitePool::connect(&database_url)
        .await
        .expect("Failed connect to database.");
    POOL.set(pool).expect("Failed to set connection pool");
}

pub async fn get_pool() -> SqlitePool {
    POOL.get().expect("Failed to get connection pool.").clone()
}
