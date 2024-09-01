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

    // let result = sqlx::query("INSERT INTO services (name, enable) VALUES (?, ?)")
    //     .bind("mimosa_milk")
    //     .bind(true)
    //     .execute(&db)
    //     .await
    //     .unwrap();

    // println!("Query result: {:?}", result);

    // let service_results = sqlx::query_as::<_, Service>("SELECT id, name, enable FROM services")
    //     .fetch_all(&db)
    //     .await
    //     .unwrap();

    // for service in service_results {
    //     println!(
    //         "[{}] name: {}, active {}",
    //         service.id, &service.name, service.enable
    //     );
    // }
}

pub async fn get_db() -> sqlx::Pool<Sqlite> {
    let db: sqlx::Pool<Sqlite> = SqlitePool::connect(DB_URL).await.unwrap();
    return db;
}
