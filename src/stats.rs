use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use actix_web::{HttpResponse, web, web::Bytes};
use serde::Serialize;
use sqlx::{Pool, Postgres};
use tokio::sync::Mutex;

use crate::error::ApiError;

const CACHE_FOR: Duration = Duration::from_secs(30);

static CACHE: LazyLock<Mutex<Option<(Instant, Bytes)>>> = LazyLock::new(Default::default);

#[derive(Serialize, sqlx::FromRow)]
struct Hour {
    hour: String,
    signed_up: i64,
    projects_created: i64,
    shipped: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct Totals {
    hours_building: f64,
    projects_created: i64,
    pairs_formed: i64,
    projects_shipped: i64,
    projects_approved: i64,
    hours_approved: f64,
    logged_in_last_day: i64,
    awaiting_review: i64,
    average_wait_hours: Option<f64>,
    average_approval_hours: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
struct Funnel {
    signed_in: i64,
    linked_hackatime: i64,
    created_project: i64,
    paired: i64,
    logged_time: i64,
    shipped: i64,
    approved: i64,
}

#[derive(Serialize)]
struct Funnels {
    day: Funnel,
    week: Funnel,
    all: Funnel,
}

#[derive(Serialize)]
struct Stats {
    hours: Vec<Hour>,
    totals: Totals,
    funnel: Funnels,
}

async fn funnel(pool: &Pool<Postgres>, hours: Option<i32>) -> sqlx::Result<Funnel> {
    sqlx::query_as(
        "SELECT
             count(*) AS signed_in,
             count(*) FILTER (WHERE h.hackatime_user_id IS NOT NULL) AS linked_hackatime,
             count(*) FILTER (WHERE EXISTS (SELECT 1 FROM project WHERE user1 = h.user_id))
                 AS created_project,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM user_project up JOIN project p USING (project_id)
                 WHERE up.user_id = h.user_id AND up.confirmed_at IS NOT NULL
                     AND (SELECT count(*) FROM user_project o
                          WHERE o.project_id = p.project_id AND o.confirmed_at IS NOT NULL) = 2
             )) AS paired,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM user_project WHERE user_id = h.user_id AND hours_logged > 0
             )) AS logged_time,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM user_project up JOIN project p USING (project_id)
                 WHERE up.user_id = h.user_id AND p.shipped_at IS NOT NULL
             )) AS shipped,
             count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM user_project up JOIN project p USING (project_id)
                 WHERE up.user_id = h.user_id AND p.approved_at IS NOT NULL
             )) AS approved
         FROM hacker h
         WHERE $1::int IS NULL
             OR h.join_at >= (now() AT TIME ZONE 'UTC') - make_interval(hours => $1)",
    )
    .bind(hours)
    .fetch_one(pool)
    .await
}

async fn build(pool: &Pool<Postgres>) -> anyhow::Result<Bytes> {
    let hours = sqlx::query_as(
        "WITH signups AS (
             SELECT date_trunc('hour', join_at) AS hour, count(*) AS n
             FROM hacker WHERE join_at IS NOT NULL GROUP BY 1
         ), created AS (
             SELECT date_trunc('hour', created_at) AS hour, count(*) AS n
             FROM project WHERE created_at IS NOT NULL GROUP BY 1
         ), shipped AS (
             SELECT date_trunc('hour', shipped_at) AS hour, count(*) AS n
             FROM project WHERE shipped_at IS NOT NULL GROUP BY 1
         ), range AS (
             SELECT generate_series(
                 LEAST((SELECT min(hour) FROM signups), date_trunc('hour', now() AT TIME ZONE 'UTC')),
                 date_trunc('hour', now() AT TIME ZONE 'UTC'),
                 interval '1 hour'
             ) AS hour
         )
         SELECT to_char(range.hour, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS hour,
             (sum(COALESCE(signups.n, 0)) OVER (ORDER BY range.hour))::bigint AS signed_up,
             (sum(COALESCE(created.n, 0)) OVER (ORDER BY range.hour))::bigint AS projects_created,
             (sum(COALESCE(shipped.n, 0)) OVER (ORDER BY range.hour))::bigint AS shipped
         FROM range
         LEFT JOIN signups USING (hour)
         LEFT JOIN created USING (hour)
         LEFT JOIN shipped USING (hour)
         ORDER BY range.hour",
    )
    .fetch_all(pool)
    .await?;
    let totals = sqlx::query_as(
        "SELECT
             (SELECT COALESCE(sum(hours_logged), 0) FROM user_project)::float8 AS hours_building,
             (SELECT count(*) FROM project) AS projects_created,
             (SELECT count(*) FROM project p
              WHERE (SELECT count(*) FROM user_project up
                     WHERE up.project_id = p.project_id AND up.confirmed_at IS NOT NULL) = 2)
                 AS pairs_formed,
             (SELECT count(*) FROM project WHERE shipped_at IS NOT NULL) AS projects_shipped,
             (SELECT count(*) FROM project WHERE approved_at IS NOT NULL) AS projects_approved,
             (SELECT COALESCE(sum(up.hours_logged), 0)
              FROM user_project up JOIN project p USING (project_id)
              WHERE p.approved_at IS NOT NULL)::float8 AS hours_approved,
             (SELECT count(*) FROM hacker
              WHERE last_login_at > (now() AT TIME ZONE 'UTC') - interval '1 day')
                 AS logged_in_last_day,
             (SELECT count(*) FROM project WHERE shipped_at IS NOT NULL AND approved_at IS NULL)
                 AS awaiting_review,
             (SELECT extract(epoch FROM avg((now() AT TIME ZONE 'UTC') - shipped_at))::float8 / 3600
              FROM project WHERE shipped_at IS NOT NULL AND approved_at IS NULL)
                 AS average_wait_hours,
             (SELECT extract(epoch FROM avg(approved_at - shipped_at))::float8 / 3600
              FROM project WHERE shipped_at IS NOT NULL AND approved_at IS NOT NULL)
                 AS average_approval_hours",
    )
    .fetch_one(pool)
    .await?;
    let funnel = Funnels {
        day: funnel(pool, Some(24)).await?,
        week: funnel(pool, Some(24 * 7)).await?,
        all: funnel(pool, None).await?,
    };
    Ok(serde_json::to_vec(&Stats { hours, totals, funnel })?.into())
}

#[actix_web::get("/stats/data")]
pub async fn get_stats(pool: web::Data<Pool<Postgres>>) -> Result<HttpResponse, ApiError> {
    let mut cache = CACHE.lock().await;
    let body = match &*cache {
        Some((built, body)) if built.elapsed() < CACHE_FOR => body.clone(),
        _ => {
            let body = build(&pool).await?;
            *cache = Some((Instant::now(), body.clone()));
            body
        }
    };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(body))
}

#[actix_web::get("/stats")]
pub async fn serve_stats() -> HttpResponse {
    match std::fs::read_to_string("./ui/build/stats.html") {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[sqlx::test]
    async fn builds_on_an_empty_database(pool: PgPool) {
        let body = super::build(&pool).await.unwrap();
        let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(stats["hours"].as_array().unwrap().len(), 1);
        assert_eq!(stats["totals"]["projects_created"], 0);
        assert_eq!(stats["totals"]["average_wait_hours"], serde_json::Value::Null);
        assert_eq!(stats["funnel"]["all"]["signed_in"], 0);
    }

    #[sqlx::test]
    async fn counts_a_shipped_pair(pool: PgPool) {
        sqlx::query(
            "INSERT INTO hacker (user_id, hca_id, join_at, last_login_at, hackatime_user_id) VALUES
                 (1, 'a', now() AT TIME ZONE 'UTC' - interval '3 hours', now() AT TIME ZONE 'UTC', 10),
                 (2, 'b', now() AT TIME ZONE 'UTC' - interval '2 hours', now() AT TIME ZONE 'UTC' - interval '2 days', NULL),
                 (3, 'c', now() AT TIME ZONE 'UTC' - interval '10 days', NULL, NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO project (project_id, user1, user2, created_at, shipped_at, approved_at) VALUES
                 (1, 1, 2, now() AT TIME ZONE 'UTC' - interval '2 hours', now() AT TIME ZONE 'UTC' - interval '1 hour', NULL),
                 (2, 3, NULL, now() AT TIME ZONE 'UTC' - interval '9 days', NULL, NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_project (project_id, user_id, confirmed_at, hours_logged) VALUES
                 (1, 1, now() AT TIME ZONE 'UTC', 5.5), (1, 2, now() AT TIME ZONE 'UTC', 4.5),
                 (2, 3, now() AT TIME ZONE 'UTC', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let body = super::build(&pool).await.unwrap();
        let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let hours = stats["hours"].as_array().unwrap();
        assert_eq!(hours.len(), 24 * 10 + 1);
        let last = hours.last().unwrap();
        assert_eq!(last["signed_up"], 3);
        assert_eq!(last["projects_created"], 2);
        assert_eq!(last["shipped"], 1);
        assert_eq!(hours[0]["signed_up"], 1);
        assert_eq!(hours[0]["projects_created"], 0);

        let totals = &stats["totals"];
        assert_eq!(totals["hours_building"], 10.0);
        assert_eq!(totals["projects_created"], 2);
        assert_eq!(totals["pairs_formed"], 1);
        assert_eq!(totals["projects_shipped"], 1);
        assert_eq!(totals["projects_approved"], 0);
        assert_eq!(totals["hours_approved"], 0.0);
        assert_eq!(totals["logged_in_last_day"], 1);
        assert_eq!(totals["awaiting_review"], 1);
        assert!((totals["average_wait_hours"].as_f64().unwrap() - 1.0).abs() < 0.01);
        assert_eq!(totals["average_approval_hours"], serde_json::Value::Null);

        let day = &stats["funnel"]["day"];
        assert_eq!(day["signed_in"], 2);
        assert_eq!(day["linked_hackatime"], 1);
        assert_eq!(day["created_project"], 1);
        assert_eq!(day["paired"], 2);
        assert_eq!(day["logged_time"], 2);
        assert_eq!(day["shipped"], 2);
        assert_eq!(day["approved"], 0);
        let all = &stats["funnel"]["all"];
        assert_eq!(all["signed_in"], 3);
        assert_eq!(all["created_project"], 2);
        assert_eq!(all["paired"], 2);
    }
}
