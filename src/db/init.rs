use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::crypto;

pub async fn init_db() -> anyhow::Result<PgPool> {
    log::info!("Initializing database");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&std::env::var("DATABASE_URL").expect("Missing variable DATABASE_URL"))
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    encrypt_tokens(&pool).await?;
    release_stuck_submissions(&pool).await?;

    Ok(pool)
}

async fn encrypt_tokens(pool: &PgPool) -> anyhow::Result<()> {
    let query = if crypto::has_previous_key() {
        "SELECT user_id, hackatime_token FROM hacker WHERE hackatime_token <> ''"
    } else {
        "SELECT user_id, hackatime_token FROM hacker WHERE hackatime_token <> '' AND hackatime_token NOT LIKE 'v2:%'"
    };
    let rows: Vec<(i32, String)> = sqlx::query_as(query).fetch_all(pool).await?;
    let mut encrypted = 0;
    let mut unreadable = 0;
    for (user_id, stored) in rows {
        let token = if crypto::is_encrypted(&stored) {
            match crypto::open(user_id, &stored) {
                Ok((_, true)) => continue,
                Ok((token, false)) => token,
                Err(_) => {
                    unreadable += 1;
                    continue;
                }
            }
        } else {
            stored.clone()
        };
        encrypted += sqlx::query(
            "UPDATE hacker SET hackatime_token = $1 WHERE user_id = $2 AND hackatime_token = $3",
        )
        .bind(crypto::encrypt(user_id, &token)?)
        .bind(user_id)
        .bind(&stored)
        .execute(pool)
        .await?
        .rows_affected();
    }
    if encrypted > 0 {
        log::info!("Re-encrypted {encrypted} stored Hackatime tokens");
    }
    if unreadable > 0 {
        log::warn!("{unreadable} stored Hackatime tokens can't be decrypted with TOKEN_KEY or TOKEN_KEY_PREVIOUS");
    }
    Ok(())
}

async fn release_stuck_submissions(pool: &PgPool) -> anyhow::Result<()> {
    let released: Vec<(i32, i32)> = sqlx::query_as(
        "UPDATE user_project SET submitting_at = NULL WHERE submitting_at IS NOT NULL RETURNING project_id, user_id",
    )
    .fetch_all(pool)
    .await?;
    for (project_id, user_id) in released {
        log::warn!(
            "Released a submission left in progress at shutdown (project {project_id}, user {user_id}); check Airtable for a duplicate record"
        );
    }
    Ok(())
}
