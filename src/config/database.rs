use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::env;

pub async fn connect() -> MySqlPool {
    // Get database url from environment variable
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must set");

    match MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("Connected to database");
            pool
        }
        Err(e) => {
            println!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    }
}
