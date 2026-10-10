use actix_web::HttpRequest;
use hc_helper::auth;
use log::info;
use rand::random_range;
use sqlx::{
    Pool, Postgres,
    types::chrono::{NaiveDateTime, Utc},
};

use crate::db::schema::{
    DBHacker, DBProject, DBUserProject, HackerParsed, ProjectParsed, UserProjectParsed,
};
use crate::slack;

pub async fn get_hacker(hca_id: String, pool: &Pool<Postgres>) -> anyhow::Result<HackerParsed> {
    let hacker: DBHacker =
        sqlx::query_as!(DBHacker, "SELECT * FROM hacker WHERE hca_id = $1", hca_id)
            .fetch_one(pool)
            .await?;
    Ok(hacker.into())
}

pub async fn get_hacker_from_user_id(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<HackerParsed> {
    let hacker: DBHacker =
        sqlx::query_as!(DBHacker, "SELECT * FROM hacker WHERE user_id = $1", user_id)
            .fetch_one(pool)
            .await?;
    Ok(hacker.into())
}

pub async fn get_hacker_from_friend_code(
    friend_code: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<HackerParsed>> {
    let hacker: Option<DBHacker> = sqlx::query_as!(
        DBHacker,
        "SELECT * FROM hacker WHERE friend_code = $1",
        friend_code
    )
    .fetch_optional(pool)
    .await?;
    Ok(hacker.map(Into::into))
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

pub async fn get_user_project(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<UserProjectParsed>> {
    let user_project: Option<DBUserProject> = sqlx::query_as!(
        DBUserProject,
        "SELECT * FROM user_project WHERE project_id = $1 AND user_id = $2",
        project_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(user_project.map(Into::into))
}

pub async fn get_users_user_project_ids(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<i32>> {
    info!("Getting user projects");
    let projects: Vec<DBUserProject> = sqlx::query_as!(
        DBUserProject,
        "SELECT * FROM user_project WHERE user_id = $1 ORDER BY project_id DESC",
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(projects
        .iter()
        .map(|x| x.project_id.unwrap_or_default())
        .collect())
}

pub async fn hackatime_names_in_use(
    user_id: i32,
    excluded_project_id: i32,
    names: &[String],
    pool: &Pool<Postgres>,
) -> anyhow::Result<bool> {
    let in_use = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_project WHERE user_id = $1 AND project_id <> $2 AND hackatime_project_names && $3)",
        user_id,
        excluded_project_id,
        names
    )
    .fetch_one(pool)
    .await?;
    Ok(in_use.unwrap_or(false))
}

pub struct ProjectFields {
    pub name: String,
    pub description: String,
    pub screenshot_url: String,
    pub code_url: String,
    pub playable_url: String,
    pub games: Vec<String>,
}

pub async fn create_project(
    fields: &ProjectFields,
    creator: i32,
    friend: Option<i32>,
    first_time: bool,
    hackatime_project_names: &[String],
    pool: &Pool<Postgres>,
) -> anyhow::Result<i32> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;

    let project_id = sqlx::query_scalar!(
        "INSERT INTO project (name, description, screenshot_url, code_url, playable_url, games, user1, user2, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING project_id",
        fields.name,
        fields.description,
        fields.screenshot_url,
        fields.code_url,
        fields.playable_url,
        &fields.games,
        creator,
        friend,
        cur_time
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO user_project (project_id, user_id, first_time, hours_needed, confirmed_at, hackatime_project_names) VALUES ($1, $2, $3, $4, $5, $6)",
        project_id,
        creator,
        first_time,
        hours_needed(first_time),
        cur_time,
        hackatime_project_names
    )
    .execute(&mut *tx)
    .await?;

    if let Some(friend) = friend {
        sqlx::query!(
            "INSERT INTO user_project (project_id, user_id, hours_needed) VALUES ($1, $2, $3)",
            project_id,
            friend,
            hours_needed(false)
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(project_id)
}

pub async fn get_partnerless_project(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<ProjectParsed>> {
    let project: Option<DBProject> = sqlx::query_as!(
        DBProject,
        "SELECT * FROM project WHERE user1 = $1 AND user2 IS NULL",
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(project.map(Into::into))
}

pub async fn join_project(
    project_id: i32,
    user_id: i32,
    first_time: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<bool> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;

    let claimed = sqlx::query!(
        "UPDATE project SET user2 = $1 WHERE project_id = $2 AND user2 IS NULL AND user1 <> $1",
        user_id,
        project_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !claimed {
        return Ok(false);
    }

    sqlx::query!(
        "INSERT INTO user_project (project_id, user_id, first_time, hours_needed, confirmed_at) VALUES ($1, $2, $3, $4, $5)",
        project_id,
        user_id,
        first_time,
        hours_needed(first_time),
        cur_time
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn get_project_members(
    project_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<HackerParsed>> {
    let hackers: Vec<DBHacker> = sqlx::query_as!(
        DBHacker,
        "SELECT hacker.* FROM hacker JOIN user_project USING (user_id) WHERE project_id = $1",
        project_id
    )
    .fetch_all(pool)
    .await?;
    Ok(hackers.into_iter().map(Into::into).collect())
}

pub async fn update_project_fields(
    project_id: i32,
    fields: &ProjectFields,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE project SET name = $1, description = $2, screenshot_url = $3, code_url = $4, playable_url = $5, games = $6 WHERE project_id = $7",
        fields.name,
        fields.description,
        fields.screenshot_url,
        fields.code_url,
        fields.playable_url,
        &fields.games,
        project_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user_project(
    project_id: i32,
    user_id: i32,
    first_time: bool,
    hackatime_project_names: &[String],
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE user_project SET first_time = $1, hours_needed = $2, hackatime_project_names = $3 WHERE project_id = $4 AND user_id = $5",
        first_time,
        hours_needed(first_time),
        hackatime_project_names,
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn confirm_user_project(
    project_id: i32,
    user_id: i32,
    first_time: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE user_project SET confirmed_at = $1, first_time = $2, hours_needed = $3 WHERE project_id = $4 AND user_id = $5",
        cur_time,
        first_time,
        hours_needed(first_time),
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_project(project_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query!("DELETE FROM project WHERE project_id = $1", project_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_ship_ok(
    project_id: i32,
    user_id: i32,
    hours_logged: f32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE user_project SET ship_ok_at = $1, hours_logged = $2 WHERE project_id = $3 AND user_id = $4",
        cur_time,
        hours_logged,
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_airtable_record_id(
    project_id: i32,
    user_id: i32,
    record_id: &str,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE user_project SET airtable_record_id = $1 WHERE project_id = $2 AND user_id = $3",
        record_id,
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_shipped(project_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE project SET shipped_at = $1 WHERE project_id = $2",
        cur_time,
        project_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_review_record_ids(pool: &Pool<Postgres>) -> anyhow::Result<Vec<(i32, String)>> {
    let rows = sqlx::query!(
        r#"SELECT up.project_id AS "project_id!", up.airtable_record_id AS "record_id!" FROM user_project up JOIN project p USING (project_id) WHERE p.shipped_at IS NOT NULL AND p.approved_at IS NULL AND up.airtable_record_id IS NOT NULL"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.project_id, r.record_id))
        .collect())
}

pub async fn set_review_status(
    project_id: i32,
    status: &str,
    approved: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE project SET review_status = $1, approved_at = CASE WHEN $2 THEN $3 ELSE approved_at END WHERE project_id = $4",
        status,
        approved,
        cur_time,
        project_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub struct ActivityRow {
    pub project_id: i32,
    pub user_id: i32,
    pub project_name: String,
    pub hackatime_token: String,
    pub slack_id: String,
    pub hackatime_project_names: Vec<String>,
    pub last_seen_hours: Option<f32>,
    pub last_active_at: Option<NaiveDateTime>,
    pub nudged_at: Option<NaiveDateTime>,
}

pub async fn get_activity_rows(pool: &Pool<Postgres>) -> anyhow::Result<Vec<ActivityRow>> {
    let rows = sqlx::query!(
        r#"SELECT up.project_id, up.user_id, p.name AS "project_name?", h.hackatime_token AS "hackatime_token!", h.slack_id AS "slack_id!", up.hackatime_project_names, up.last_seen_hours, up.last_active_at, up.nudged_at
        FROM user_project up
        JOIN project p USING (project_id)
        JOIN hacker h USING (user_id)
        WHERE up.confirmed_at IS NOT NULL AND up.ship_ok_at IS NULL AND p.shipped_at IS NULL AND NOT h.reminders_opt_out
            AND h.hackatime_token IS NOT NULL AND h.hackatime_token <> '' AND h.slack_id IS NOT NULL AND h.slack_id <> ''"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ActivityRow {
            project_id: r.project_id,
            user_id: r.user_id,
            project_name: r.project_name.unwrap_or_default(),
            hackatime_token: r.hackatime_token,
            slack_id: r.slack_id,
            hackatime_project_names: r.hackatime_project_names,
            last_seen_hours: r.last_seen_hours,
            last_active_at: r.last_active_at,
            nudged_at: r.nudged_at,
        })
        .collect())
}

pub async fn set_activity(
    project_id: i32,
    user_id: i32,
    hours: f32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE user_project SET last_seen_hours = $1, last_active_at = $2, nudged_at = NULL WHERE project_id = $3 AND user_id = $4",
        hours,
        cur_time,
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_nudged(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query!(
        "UPDATE user_project SET nudged_at = $1 WHERE project_id = $2 AND user_id = $3",
        cur_time,
        project_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_reminders_opt_out(
    user_id: i32,
    opt_out: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE hacker SET reminders_opt_out = $1 WHERE user_id = $2",
        opt_out,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn hours_needed(first_time: bool) -> f32 {
    if first_time { 3.0 } else { 5.0 }
}

pub async fn on_login_push_or_update_hacker(
    hca_token: String,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    info!("Handling database for new login");
    let auth_data = auth::data::get_auth_data_with_token(&hca_token).await?;
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
                "UPDATE hacker SET last_login_at = $1, hca_token = $2, slack_id = $3 WHERE hca_id = $4",
                cur_time,
                hca_token,
                auth_data.slack_id,
                auth_id
            )
            .execute(pool)
            .await?;
        }
        false => {
            let mut friend_code = 0;

            while friend_code == 0
                || sqlx::query_scalar!(
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
            .bind(&auth_data.slack_id)
            .bind(friend_code)
            .execute(pool)
            .await?;

            slack::dm(
                &auth_data.slack_id,
                "Thank you for signing up to Hack Club Dyad, looking forward to seeing what you create!".into(),
            );
        }
    }

    Ok(())
}

pub async fn on_link_hackatime(
    hackatime_token: String,
    req: &HttpRequest,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
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
