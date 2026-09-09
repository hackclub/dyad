use actix_web::HttpRequest;
use hc_helper::{auth, hackatime};
use rand::random_range;
use sqlx::{
    Pool, Postgres,
    types::chrono::{NaiveDateTime, Utc},
};
use log::info;

use crate::db::schema::{
    DBHacker, DBProject, DBUserProject, HackerParsed, ProjectParsed, UserProjectParsed,
};

pub async fn get_hacker(hca_id: String, pool: &Pool<Postgres>) -> anyhow::Result<HackerParsed> {
    let hacker: DBHacker =
        sqlx::query_as!(DBHacker, "SELECT * FROM hacker WHERE hca_id = $1", hca_id)
            .fetch_one(pool)
            .await?;
    Ok(hacker.into())
}

pub async fn get_project(project_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<ProjectParsed> {
    let project: DBProject = sqlx::query_as!(
        DBProject,
        "SELECT * FROM project WHERE project_id = $1",
        project_id
    )
    .fetch_one(pool)
    .await?;
    Ok(project.into())
}

pub async fn get_user_project(project_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<UserProjectParsed> {
    let user_project: DBUserProject = sqlx::query_as!(
        DBUserProject,
        "SELECT * FROM user_project WHERE project_id = $1",
        project_id
    )
    .fetch_one(pool)
    .await?;
    Ok(user_project.into())
}

pub async fn get_users_user_project_ids(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<i32>> {
    info!("Getting user projects");
    let projects: Vec<DBUserProject> = sqlx::query_as!(
        DBUserProject,
        "SELECT * FROM user_project WHERE user_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(projects
        .iter()
        .map(|x| x.project_id.unwrap_or_default())
        .collect())
}

pub async fn on_login_push_or_update_hacker(
    hca_token: String,
    req: &HttpRequest,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    info!("Handling database for new login");
    let auth_data = auth::data::get_auth_data(req).await?;
    let auth_id = auth_data.id;

    let user_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM hacker WHERE hca_id = $1)",
        auth_id
    )
    .fetch_one(pool)
    .await?
    .ok_or("couldn't select from hacker table")
    .map_err(|e| anyhow::anyhow!(e))?;

    let cur_time: NaiveDateTime = Utc::now().naive_utc();

    match user_exists {
        true => {
            sqlx::query!(
                "UPDATE hacker SET last_login_at = $1 WHERE hca_id = $2",
                cur_time,
                auth_id
            )
            .execute(pool)
            .await?;
        }
        false => {
            let mut friend_code = 0;

            while friend_code == 0 || sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM hacker WHERE friend_code = $1)",
                friend_code
            )
            .fetch_one(pool)
            .await?
            .ok_or("couldn't select from hacker table")
            .map_err(|e| anyhow::anyhow!(e))?
            {
                friend_code = random_range(100000..=999999);
            }

            sqlx::query("INSERT INTO hacker (hca_id, email, first_name, hca_token, join_at, last_login_at, slack_id, friend_code) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(auth_id.clone())
            .bind(auth_data.primary_email)
            .bind(auth_data.first_name)
            .bind(hca_token)
            .bind(cur_time)
            .bind(cur_time)
            .bind(auth_data.slack_id)
            .bind(friend_code)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn get_hackatime_token_from_db(
    hca_id: String,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<String>> {
    info!("Getting DB-store Hackatime token");
    let hackatime_token = sqlx::query_scalar!(
        "SELECT hackatime_token FROM hacker WHERE hca_id = $1",
        hca_id
    )
    .fetch_one(pool)
    .await?;

    Ok(hackatime_token)
}

pub async fn on_link_hackatime(hackatime_token: String, req: &HttpRequest, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    info!("Hackatime link started");
    let auth_data = auth::data::get_auth_data(req).await?;
    let auth_id = auth_data.id;

    sqlx::query!(
        "UPDATE hacker SET hackatime_token = $1 WHERE hca_id = $2",
        hackatime_token,
        auth_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

