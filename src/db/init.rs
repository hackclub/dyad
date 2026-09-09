use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn init_db() -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL").expect("Missing variable DATABASE_URL"))
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
