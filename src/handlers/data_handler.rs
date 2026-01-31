use crate::config::database;
use bcrypt::hash;
use futures::{StreamExt, stream};
use sqlx::{Error, MySqlPool, QueryBuilder};
use std::time::Instant;

const TOTAL_ROWS: i64 = 10_000_000;
const BATCH_SIZE: i64 = 10_000;
const CONCURRENCY: usize = 16;

pub async fn bulk_import() -> Result<String, Error> {
    let db = database::connect().await;

    let start = Instant::now();

    let total_batches = TOTAL_ROWS / BATCH_SIZE;

    stream::iter(0..total_batches)
        .for_each_concurrent(CONCURRENCY, |batch_index| {
            let pool = db.clone();
            async move {
                insert_batch(&pool, batch_index).await.unwrap();
            }
        })
        .await;

    let elapsed = start.elapsed();
    let total_time = format!("Total time: {} seconds", elapsed.as_secs_f64());

    Ok(total_time)
}

async fn insert_batch(pool: &MySqlPool, batch_index: i64) -> Result<(), Error> {
    let password = "123456";
    let hashed = match hash(password, 10) {
        Ok(hashed) => hashed,
        Err(e) => {
            println!("Failed to hash password: {}", e);
            "password".to_string()
        }
    };

    let start_id = batch_index * BATCH_SIZE;

    let mut tx = pool.begin().await?;

    let mut builder = QueryBuilder::new("INSERT INTO users (name, email, password) ");

    builder.push_values(0..BATCH_SIZE, |mut b, i| {
        b.push_bind(format!("User{}", i + start_id))
            .push_bind(format!("user{}@example.com", i + start_id))
            .push_bind(&hashed);
    });

    builder.build().execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(())
}
