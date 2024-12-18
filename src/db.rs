use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

const DB_URL: &str = "sqlite://db/sqlite.db";

pub async fn init() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db: sqlx::Pool<_> = SqlitePool::connect(DB_URL).await.unwrap();

    let result = sqlx::query(include_str!("../db/creation.sql"))
        .execute(&db)
        .await
        .unwrap();
    println!("DB creation result: {:?}", result);
}

pub async fn get_db() -> sqlx::Pool<Sqlite> {
    let db: sqlx::Pool<Sqlite> = SqlitePool::connect(DB_URL).await.unwrap();
    db
}
