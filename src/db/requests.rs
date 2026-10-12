use std::time::{Duration, SystemTime};

use actix_web::HttpRequest;
use anyhow::bail;
use hc_helper::{auth, hackatime::data::get_hackatime_user_with_token};
use log::info;
use rand::random_range;
use sqlx::{
    Acquire, PgConnection, Pool, Postgres,
    types::chrono::{DateTime, NaiveDateTime, Utc},
};

use crate::db::schema::{
    DBHacker, DBProject, DBUserProject, HackerParsed, ProjectParsed, UserProjectParsed,
};
use crate::{crypto, slack};

pub async fn get_hacker(hca_id: String, pool: &Pool<Postgres>) -> anyhow::Result<HackerParsed> {
    let hacker: DBHacker = sqlx::query_as("SELECT * FROM hacker WHERE hca_id = $1")
        .bind(hca_id)
        .fetch_one(pool)
        .await?;
    Ok(hacker.into())
}

pub async fn get_hacker_from_user_id(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<HackerParsed> {
    let hacker: DBHacker = sqlx::query_as("SELECT * FROM hacker WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(hacker.into())
}

pub async fn get_hacker_from_invite_token(
    invite_token: &str,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<HackerParsed>> {
    let hacker: Option<DBHacker> = sqlx::query_as("SELECT * FROM hacker WHERE invite_token = $1")
        .bind(invite_token)
        .fetch_optional(pool)
        .await?;
    Ok(hacker.map(Into::into))
}

pub async fn get_hackers(
    user_ids: &[i32],
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<HackerParsed>> {
    let hackers: Vec<DBHacker> = sqlx::query_as("SELECT * FROM hacker WHERE user_id = ANY($1)")
        .bind(user_ids)
        .fetch_all(pool)
        .await?;
    Ok(hackers.into_iter().map(Into::into).collect())
}

pub async fn get_project(project_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<ProjectParsed> {
    let project: DBProject = sqlx::query_as("SELECT * FROM project WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(pool)
        .await?;
    Ok(project.into())
}

pub async fn get_user_project(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<UserProjectParsed>> {
    let user_project: Option<DBUserProject> =
        sqlx::query_as("SELECT * FROM user_project WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(user_project.map(Into::into))
}

pub async fn get_projects_for_user(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<ProjectParsed>> {
    let projects: Vec<DBProject> = sqlx::query_as(
        "SELECT p.* FROM project p JOIN user_project up USING (project_id) WHERE up.user_id = $1 ORDER BY p.project_id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(projects.into_iter().map(Into::into).collect())
}

pub async fn get_user_projects(
    project_ids: &[i32],
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<UserProjectParsed>> {
    let user_projects: Vec<DBUserProject> =
        sqlx::query_as("SELECT * FROM user_project WHERE project_id = ANY($1)")
            .bind(project_ids)
            .fetch_all(pool)
            .await?;
    Ok(user_projects.into_iter().map(Into::into).collect())
}

fn violates(e: &sqlx::Error, constraint: &str) -> bool {
    e.as_database_error().and_then(|e| e.constraint()) == Some(constraint)
}

async fn lock_project(project_id: i32, conn: &mut PgConnection) -> anyhow::Result<bool> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT project_id FROM project WHERE project_id = $1 AND shipped_at IS NULL FOR UPDATE",
    )
    .bind(project_id)
    .fetch_optional(conn)
    .await?;
    Ok(found.is_some())
}

async fn names_in_use(
    user_id: i32,
    excluded_project_id: i32,
    names: &[String],
    conn: &mut PgConnection,
) -> anyhow::Result<bool> {
    sqlx::query("SELECT 1 FROM hacker WHERE user_id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    let in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_project WHERE user_id = $1 AND project_id <> $2 AND hackatime_project_names && $3)",
    )
    .bind(user_id)
    .bind(excluded_project_id)
    .bind(names)
    .fetch_one(conn)
    .await?;
    Ok(in_use)
}

pub struct ProjectFields {
    pub name: String,
    pub description: String,
    pub screenshot_url: String,
    pub code_url: String,
    pub playable_url: String,
    pub games: Vec<String>,
}

pub const MAX_PENDING_INVITES: i64 = 3;

const PENDING_INVITES: &str = "SELECT count(*) FROM project p JOIN user_project up ON up.project_id = p.project_id AND up.user_id = p.user2 WHERE p.user1 = $1 AND up.confirmed_at IS NULL";

pub async fn has_too_many_invites(user_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<bool> {
    let pending: i64 = sqlx::query_scalar(PENDING_INVITES)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(pending >= MAX_PENDING_INVITES)
}

pub enum CreateOutcome {
    Created(i32),
    NamesInUse,
    PartnerlessTaken,
    TooManyInvites,
}

pub async fn create_project(
    fields: &ProjectFields,
    creator: i32,
    friend: Option<i32>,
    first_time: bool,
    hackatime_project_names: &[String],
    pool: &Pool<Postgres>,
) -> anyhow::Result<CreateOutcome> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;

    if names_in_use(creator, 0, hackatime_project_names, &mut tx).await? {
        return Ok(CreateOutcome::NamesInUse);
    }
    if friend.is_some() {
        let pending: i64 = sqlx::query_scalar(PENDING_INVITES)
            .bind(creator)
            .fetch_one(&mut *tx)
            .await?;
        if pending >= MAX_PENDING_INVITES {
            return Ok(CreateOutcome::TooManyInvites);
        }
    }

    let inserted = sqlx::query_scalar(
        "INSERT INTO project (name, description, screenshot_url, code_url, playable_url, games, user1, user2, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING project_id",
    )
    .bind(&fields.name)
    .bind(&fields.description)
    .bind(&fields.screenshot_url)
    .bind(&fields.code_url)
    .bind(&fields.playable_url)
    .bind(&fields.games)
    .bind(creator)
    .bind(friend)
    .bind(cur_time)
    .fetch_one(&mut *tx)
    .await;
    let project_id: i32 = match inserted {
        Err(e) if violates(&e, "project_one_partnerless") => {
            return Ok(CreateOutcome::PartnerlessTaken);
        }
        r => r?,
    };

    sqlx::query("INSERT INTO user_project (project_id, user_id, first_time, hours_needed, confirmed_at, hackatime_project_names) VALUES ($1, $2, $3, $4, $5, $6)")
    .bind(project_id)
    .bind(creator)
    .bind(first_time)
    .bind(hours_needed(first_time))
    .bind(cur_time)
    .bind(hackatime_project_names)
    .execute(&mut *tx)
    .await?;

    if let Some(friend) = friend {
        sqlx::query(
            "INSERT INTO user_project (project_id, user_id, hours_needed) VALUES ($1, $2, $3)",
        )
        .bind(project_id)
        .bind(friend)
        .bind(hours_needed(false))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(CreateOutcome::Created(project_id))
}

pub enum SaveOutcome {
    Saved,
    Locked,
    NamesInUse,
}

pub async fn save_project(
    project_id: i32,
    user_id: i32,
    fields: Option<&ProjectFields>,
    mine: Option<(bool, &[String])>,
    pool: &Pool<Postgres>,
) -> anyhow::Result<SaveOutcome> {
    let mut tx = pool.begin().await?;
    if !lock_project(project_id, &mut tx).await? {
        return Ok(SaveOutcome::Locked);
    }

    if let Some((first_time, names)) = mine {
        if names_in_use(user_id, project_id, names, &mut tx).await? {
            return Ok(SaveOutcome::NamesInUse);
        }
        let updated = sqlx::query("UPDATE user_project SET first_time = $1, hours_needed = $2, hackatime_project_names = $3 WHERE project_id = $4 AND user_id = $5 AND ship_ok_at IS NULL AND (submitting_at IS NULL OR submitting_at <= $6)")
        .bind(first_time)
        .bind(hours_needed(first_time))
        .bind(names)
        .bind(project_id)
        .bind(user_id)
        .bind(submit_cutoff())
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if !updated {
            return Ok(SaveOutcome::Locked);
        }
    }

    if let Some(fields) = fields {
        let updated = sqlx::query("UPDATE project SET name = $1, description = $2, screenshot_url = $3, code_url = $4, playable_url = $5, games = $6 WHERE project_id = $7 AND NOT EXISTS (SELECT 1 FROM user_project WHERE project_id = $7 AND (ship_ok_at IS NOT NULL OR submitting_at > $8))")
        .bind(&fields.name)
        .bind(&fields.description)
        .bind(&fields.screenshot_url)
        .bind(&fields.code_url)
        .bind(&fields.playable_url)
        .bind(&fields.games)
        .bind(project_id)
        .bind(submit_cutoff())
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if !updated {
            return Ok(SaveOutcome::Locked);
        }
    }

    tx.commit().await?;
    Ok(SaveOutcome::Saved)
}

pub async fn get_partnerless_project(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<ProjectParsed>> {
    let project: Option<DBProject> =
        sqlx::query_as("SELECT * FROM project WHERE user1 = $1 AND user2 IS NULL")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(project.map(Into::into))
}

pub enum JoinOutcome {
    Requested,
    AlreadyRequested,
    Unavailable,
}

pub async fn request_join(
    project_id: i32,
    user_id: i32,
    first_time: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<JoinOutcome> {
    let inserted = sqlx::query(
        "INSERT INTO join_request (project_id, user_id, first_time) SELECT $1, $2, $3 WHERE EXISTS (SELECT 1 FROM project WHERE project_id = $1 AND user2 IS NULL AND user1 <> $2 FOR SHARE) ON CONFLICT DO NOTHING",
    )
    .bind(project_id)
    .bind(user_id)
    .bind(first_time)
    .execute(pool)
    .await?
    .rows_affected()
        == 1;
    if inserted {
        return Ok(JoinOutcome::Requested);
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM join_request WHERE project_id = $1 AND user_id = $2)",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(if exists {
        JoinOutcome::AlreadyRequested
    } else {
        JoinOutcome::Unavailable
    })
}

pub async fn cancel_join(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM join_request WHERE project_id = $1 AND user_id = $2")
        .bind(project_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct IncomingJoinRow {
    pub project_id: i32,
    pub user_id: i32,
    pub name: String,
}

pub async fn get_incoming_join_requests(
    owner_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<IncomingJoinRow>> {
    Ok(sqlx::query_as(
        "SELECT jr.project_id, jr.user_id, COALESCE(h.first_name, '') AS name
        FROM join_request jr
        JOIN project p USING (project_id)
        JOIN hacker h ON h.user_id = jr.user_id
        WHERE p.user1 = $1 AND p.user2 IS NULL
        ORDER BY jr.requested_at",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?)
}

#[derive(sqlx::FromRow)]
pub struct OutgoingJoinRow {
    pub project_id: i32,
    pub project_name: String,
    pub owner_name: String,
}

pub async fn get_outgoing_join_requests(
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<OutgoingJoinRow>> {
    Ok(sqlx::query_as(
        "SELECT jr.project_id, COALESCE(p.name, '') AS project_name, COALESCE(h.first_name, '') AS owner_name
        FROM join_request jr
        JOIN project p USING (project_id)
        JOIN hacker h ON h.user_id = p.user1
        WHERE jr.user_id = $1
        ORDER BY jr.requested_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub async fn approve_join(
    project_id: i32,
    owner_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<Vec<i32>>> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;

    let claimed = sqlx::query(
        "UPDATE project SET user2 = $3 WHERE project_id = $1 AND user1 = $2 AND user2 IS NULL AND shipped_at IS NULL",
    )
    .bind(project_id)
    .bind(owner_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !claimed {
        return Ok(None);
    }

    let first_time: Option<bool> = sqlx::query_scalar(
        "DELETE FROM join_request WHERE project_id = $1 AND user_id = $2 RETURNING first_time",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(first_time) = first_time else {
        return Ok(None);
    };

    sqlx::query("INSERT INTO user_project (project_id, user_id, first_time, hours_needed, confirmed_at) VALUES ($1, $2, $3, $4, $5)")
    .bind(project_id)
    .bind(user_id)
    .bind(first_time)
    .bind(hours_needed(first_time))
    .bind(cur_time)
    .execute(&mut *tx)
    .await?;

    let others: Vec<i32> =
        sqlx::query_scalar("DELETE FROM join_request WHERE project_id = $1 RETURNING user_id")
            .bind(project_id)
            .fetch_all(&mut *tx)
            .await?;

    tx.commit().await?;
    Ok(Some(others))
}

pub async fn decline_join(
    project_id: i32,
    owner_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<bool> {
    Ok(sqlx::query(
        "DELETE FROM join_request jr USING project p WHERE jr.project_id = $1 AND jr.user_id = $3 AND p.project_id = jr.project_id AND p.user1 = $2",
    )
    .bind(project_id)
    .bind(owner_id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

pub async fn get_project_members(
    project_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Vec<HackerParsed>> {
    let hackers: Vec<DBHacker> = sqlx::query_as(
        "SELECT hacker.* FROM hacker JOIN user_project USING (user_id) WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(hackers.into_iter().map(Into::into).collect())
}

pub async fn confirm_user_project(
    project_id: i32,
    user_id: i32,
    first_time: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<bool> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;
    if !lock_project(project_id, &mut tx).await? {
        return Ok(false);
    }
    let confirmed = sqlx::query("UPDATE user_project SET confirmed_at = $1, first_time = $2, hours_needed = $3 WHERE project_id = $4 AND user_id = $5 AND confirmed_at IS NULL")
    .bind(cur_time)
    .bind(first_time)
    .bind(hours_needed(first_time))
    .bind(project_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    tx.commit().await?;
    Ok(confirmed)
}

pub async fn decline_invite(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<bool>> {
    let mut tx = pool.begin().await?;
    if !lock_project(project_id, &mut tx).await? {
        return Ok(None);
    }

    let removed = sqlx::query(
        "DELETE FROM user_project WHERE project_id = $1 AND user_id = $2 AND confirmed_at IS NULL",
    )
    .bind(project_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !removed {
        return Ok(None);
    }

    let mut savepoint = tx.begin().await?;
    let reopened = sqlx::query("UPDATE project SET user2 = NULL WHERE project_id = $1 AND user2 = $2")
        .bind(project_id)
        .bind(user_id)
        .execute(&mut *savepoint)
        .await;
    let kept = match reopened {
        Err(e) if violates(&e, "project_one_partnerless") => {
            savepoint.rollback().await?;
            false
        }
        r => {
            let kept = r?.rows_affected() == 1;
            savepoint.commit().await?;
            kept
        }
    };
    if !kept {
        sqlx::query("DELETE FROM project WHERE project_id = $1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(Some(kept))
}

pub struct DeletedProject {
    pub name: String,
    pub invitee: Option<i32>,
    pub requesters: Vec<i32>,
}

pub async fn delete_project(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<DeletedProject>> {
    let mut tx = pool.begin().await?;
    let found: Option<(Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT name, user2 FROM project WHERE project_id = $1 AND user1 = $2 AND shipped_at IS NULL FOR UPDATE",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((name, invitee)) = found else {
        return Ok(None);
    };

    if let Some(invitee) = invitee {
        let accepted: bool = sqlx::query_scalar(
            "SELECT confirmed_at IS NOT NULL FROM user_project WHERE project_id = $1 AND user_id = $2",
        )
        .bind(project_id)
        .bind(invitee)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(false);
        if accepted {
            return Ok(None);
        }
    }
    let requesters: Vec<i32> =
        sqlx::query_scalar("SELECT user_id FROM join_request WHERE project_id = $1")
            .bind(project_id)
            .fetch_all(&mut *tx)
            .await?;

    sqlx::query("DELETE FROM project WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Some(DeletedProject {
        name: name.unwrap_or_default(),
        invitee,
        requesters,
    }))
}

const SUBMIT_TIMEOUT: Duration = Duration::from_secs(20 * 60);

fn ago(duration: Duration) -> NaiveDateTime {
    DateTime::<Utc>::from(SystemTime::now() - duration).naive_utc()
}

fn submit_cutoff() -> NaiveDateTime {
    ago(SUBMIT_TIMEOUT)
}

pub fn is_submitting(submitting_at: Option<NaiveDateTime>) -> bool {
    submitting_at.is_some_and(|at| at > submit_cutoff())
}

pub enum ClaimOutcome {
    Claimed(NaiveDateTime),
    Shipped,
    AlreadyReady,
    Busy,
    FirstTimeUsed,
}

pub async fn claim_submission(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<ClaimOutcome> {
    let now = Utc::now().naive_utc();
    let cutoff = submit_cutoff();
    let mut tx = pool.begin().await?;
    if !lock_project(project_id, &mut tx).await? {
        return Ok(ClaimOutcome::Shipped);
    }
    sqlx::query("SELECT 1 FROM hacker WHERE user_id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let (record_id, submitting_at, first_time): (Option<String>, Option<NaiveDateTime>, bool) =
        sqlx::query_as(
            "SELECT airtable_record_id, submitting_at, first_time FROM user_project WHERE project_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    if record_id.is_some() {
        return Ok(ClaimOutcome::AlreadyReady);
    }
    if submitting_at.is_some_and(|at| at > cutoff) {
        return Ok(ClaimOutcome::Busy);
    }
    if first_time {
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_project WHERE user_id = $1 AND project_id <> $2 AND first_time AND (airtable_record_id IS NOT NULL OR submitting_at > $3))",
        )
        .bind(user_id)
        .bind(project_id)
        .bind(cutoff)
        .fetch_one(&mut *tx)
        .await?;
        if used {
            return Ok(ClaimOutcome::FirstTimeUsed);
        }
    }

    let claim: NaiveDateTime = sqlx::query_scalar(
        "UPDATE user_project SET submitting_at = $1 WHERE project_id = $2 AND user_id = $3 RETURNING submitting_at",
    )
    .bind(now)
    .bind(project_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ClaimOutcome::Claimed(claim))
}

pub async fn release_submission(
    project_id: i32,
    user_id: i32,
    claim: NaiveDateTime,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE user_project SET submitting_at = NULL WHERE project_id = $1 AND user_id = $2 AND submitting_at = $3",
    )
    .bind(project_id)
    .bind(user_id)
    .bind(claim)
    .execute(pool)
    .await?;
    Ok(())
}

pub struct Recorded {
    pub shipped: bool,
    pub record_ids: Vec<String>,
}

pub async fn finish_submission(
    project_id: i32,
    user_id: i32,
    claim: NaiveDateTime,
    record_id: &str,
    hours_logged: f32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<Recorded>> {
    let now = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM project WHERE project_id = $1 FOR UPDATE")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    let recorded = sqlx::query(
        "UPDATE user_project SET airtable_record_id = $1, ship_ok_at = $2, hours_logged = $3, submitting_at = NULL WHERE project_id = $4 AND user_id = $5 AND submitting_at = $6 AND airtable_record_id IS NULL",
    )
    .bind(record_id)
    .bind(now)
    .bind(hours_logged)
    .bind(project_id)
    .bind(user_id)
    .bind(claim)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !recorded {
        return Ok(None);
    }

    let record_ids: Vec<Option<String>> =
        sqlx::query_scalar("SELECT airtable_record_id FROM user_project WHERE project_id = $1")
            .bind(project_id)
            .fetch_all(&mut *tx)
            .await?;
    let shipped = record_ids.len() >= 2
        && record_ids.iter().all(Option::is_some)
        && sqlx::query(
            "UPDATE project SET shipped_at = $1 WHERE project_id = $2 AND shipped_at IS NULL",
        )
        .bind(now)
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;

    tx.commit().await?;
    Ok(Some(Recorded {
        shipped,
        record_ids: record_ids.into_iter().flatten().collect(),
    }))
}

pub enum WithdrawOutcome {
    NotReady,
    Shipped,
    Busy,
    Withdrawn(Option<(String, NaiveDateTime)>),
}

pub async fn withdraw_submission(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<WithdrawOutcome> {
    let now = Utc::now().naive_utc();
    let mut tx = pool.begin().await?;
    if !lock_project(project_id, &mut tx).await? {
        return Ok(WithdrawOutcome::Shipped);
    }
    let (record_id, ship_ok_at, submitting_at): (
        Option<String>,
        Option<NaiveDateTime>,
        Option<NaiveDateTime>,
    ) = sqlx::query_as(
        "SELECT airtable_record_id, ship_ok_at, submitting_at FROM user_project WHERE project_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if is_submitting(submitting_at) {
        return Ok(WithdrawOutcome::Busy);
    }
    if record_id.is_none() && ship_ok_at.is_none() {
        return Ok(WithdrawOutcome::NotReady);
    }

    let claim: Option<NaiveDateTime> = sqlx::query_scalar(
        "UPDATE user_project SET airtable_record_id = NULL, ship_ok_at = NULL, submitting_at = $1 WHERE project_id = $2 AND user_id = $3 RETURNING submitting_at",
    )
    .bind(record_id.is_some().then_some(now))
    .bind(project_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(WithdrawOutcome::Withdrawn(record_id.zip(claim)))
}

#[derive(sqlx::FromRow)]
pub struct ReviewRow {
    pub project_id: i32,
    pub user_id: i32,
    pub record_id: String,
    pub shipped: bool,
}

pub async fn get_review_rows(pool: &Pool<Postgres>) -> anyhow::Result<Vec<ReviewRow>> {
    Ok(sqlx::query_as(
        "SELECT up.project_id, up.user_id, up.airtable_record_id AS record_id, p.shipped_at IS NOT NULL AS shipped FROM user_project up JOIN project p USING (project_id) WHERE p.approved_at IS NULL AND up.airtable_record_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await?)
}

pub struct SentBack {
    pub shipped: bool,
    pub sent_back: Vec<i32>,
    pub unreadied: Vec<i32>,
    pub withdrawn_records: Vec<String>,
}

pub async fn send_back(
    project_id: i32,
    missing: &[(i32, String)],
    pool: &Pool<Postgres>,
) -> anyhow::Result<Option<SentBack>> {
    let mut tx = pool.begin().await?;
    let shipped: Option<bool> = sqlx::query_scalar(
        "SELECT shipped_at IS NOT NULL FROM project WHERE project_id = $1 AND approved_at IS NULL FOR UPDATE",
    )
    .bind(project_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(shipped) = shipped else {
        return Ok(None);
    };

    let mut sent_back = vec![];
    for (user_id, record_id) in missing {
        let cleared = sqlx::query(
            "UPDATE user_project SET airtable_record_id = NULL, ship_ok_at = NULL WHERE project_id = $1 AND user_id = $2 AND airtable_record_id = $3",
        )
        .bind(project_id)
        .bind(user_id)
        .bind(record_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if cleared {
            sent_back.push(*user_id);
        }
    }
    if sent_back.is_empty() {
        return Ok(None);
    }

    let partners: Vec<(i32, Option<String>)> = sqlx::query_as(
        "SELECT user_id, airtable_record_id FROM user_project WHERE project_id = $1 AND NOT (user_id = ANY($2)) AND (airtable_record_id IS NOT NULL OR ship_ok_at IS NOT NULL OR submitting_at IS NOT NULL) FOR UPDATE",
    )
    .bind(project_id)
    .bind(&sent_back)
    .fetch_all(&mut *tx)
    .await?;
    let unreadied: Vec<i32> = partners.iter().map(|(user_id, _)| *user_id).collect();
    sqlx::query(
        "UPDATE user_project SET airtable_record_id = NULL, ship_ok_at = NULL, submitting_at = NULL WHERE project_id = $1 AND user_id = ANY($2)",
    )
    .bind(project_id)
    .bind(&unreadied)
    .execute(&mut *tx)
    .await?;

    if shipped {
        sqlx::query("UPDATE project SET shipped_at = NULL, review_status = NULL WHERE project_id = $1")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Some(SentBack {
        shipped,
        sent_back,
        unreadied,
        withdrawn_records: partners.into_iter().filter_map(|(_, r)| r).collect(),
    }))
}

pub async fn set_review_status(
    project_id: i32,
    status: &str,
    approved: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query("UPDATE project SET review_status = $1, approved_at = CASE WHEN $2 THEN $3 ELSE approved_at END WHERE project_id = $4")
    .bind(status)
    .bind(approved)
    .bind(cur_time)
    .bind(project_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct ActivityRow {
    pub project_id: i32,
    pub user_id: i32,
    pub project_name: String,
    pub hackatime_token: String,
    pub slack_id: String,
    pub hackatime_project_names: Vec<String>,
    pub confirmed_at: NaiveDateTime,
    pub nudged_at: Option<NaiveDateTime>,
}

pub async fn get_activity_rows(pool: &Pool<Postgres>) -> anyhow::Result<Vec<ActivityRow>> {
    let rows: Vec<ActivityRow> = sqlx::query_as(
        "SELECT up.project_id, up.user_id, COALESCE(p.name, '') AS project_name, h.hackatime_token, h.slack_id, up.hackatime_project_names, up.confirmed_at, up.nudged_at
        FROM user_project up
        JOIN project p USING (project_id)
        JOIN hacker h USING (user_id)
        WHERE up.confirmed_at IS NOT NULL AND up.ship_ok_at IS NULL AND p.shipped_at IS NULL AND NOT h.reminders_opt_out
            AND p.user2 IS NOT NULL
            AND NOT EXISTS (SELECT 1 FROM user_project o WHERE o.project_id = up.project_id AND o.confirmed_at IS NULL)
            AND h.hackatime_user_id IS NOT NULL
            AND h.hackatime_token IS NOT NULL AND h.hackatime_token <> '' AND h.slack_id IS NOT NULL AND h.slack_id <> ''",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ActivityRow {
            hackatime_token: crypto::decrypt_or_empty(row.user_id, Some(row.hackatime_token)),
            ..row
        })
        .filter(|row| !row.hackatime_token.is_empty())
        .collect())
}

pub async fn set_nudged(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let cur_time: NaiveDateTime = Utc::now().naive_utc();
    sqlx::query("UPDATE user_project SET nudged_at = $1 WHERE project_id = $2 AND user_id = $3")
        .bind(cur_time)
        .bind(project_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_reminders_opt_out(
    user_id: i32,
    opt_out: bool,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE hacker SET reminders_opt_out = $1 WHERE user_id = $2")
        .bind(opt_out)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn regenerate_invite_token(user_id: i32, pool: &Pool<Postgres>) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar(
        "UPDATE hacker SET invite_token = replace(gen_random_uuid()::text, '-', '') WHERE user_id = $1 RETURNING invite_token",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

pub const WRONG_CODE_LIMIT: i64 = 10;

fn wrong_code_window_start() -> NaiveDateTime {
    ago(Duration::from_secs(60 * 60))
}

pub enum FriendLookup {
    Found(HackerParsed),
    NotFound,
    TooManyAttempts,
}

pub async fn find_friend(
    user_id: i32,
    friend_code: i32,
    pool: &Pool<Postgres>,
) -> anyhow::Result<FriendLookup> {
    let window_start = wrong_code_window_start();
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM hacker WHERE user_id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let attempts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM wrong_friend_code WHERE user_id = $1 AND attempted_at > $2",
    )
    .bind(user_id)
    .bind(window_start)
    .fetch_one(&mut *tx)
    .await?;
    if attempts >= WRONG_CODE_LIMIT {
        return Ok(FriendLookup::TooManyAttempts);
    }

    let friend: Option<DBHacker> = sqlx::query_as("SELECT * FROM hacker WHERE friend_code = $1")
        .bind(friend_code)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(friend) = friend {
        return Ok(FriendLookup::Found(friend.into()));
    }

    sqlx::query("INSERT INTO wrong_friend_code (user_id, attempted_at) VALUES ($1, $2)")
        .bind(user_id)
        .bind(Utc::now().naive_utc())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM wrong_friend_code WHERE attempted_at <= $1")
        .bind(window_start)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(FriendLookup::NotFound)
}

const DM_LIMIT: i64 = 3;

pub async fn allow_dm(sender: i32, recipient: i32, pool: &Pool<Postgres>) -> anyhow::Result<bool> {
    let window_start = ago(Duration::from_secs(60 * 60));
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM hacker WHERE user_id = $1 FOR UPDATE")
        .bind(sender)
        .execute(&mut *tx)
        .await?;
    let sent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM dm_log WHERE sender = $1 AND recipient = $2 AND sent_at > $3",
    )
    .bind(sender)
    .bind(recipient)
    .bind(window_start)
    .fetch_one(&mut *tx)
    .await?;
    if sent >= DM_LIMIT {
        return Ok(false);
    }

    sqlx::query("INSERT INTO dm_log (sender, recipient, sent_at) VALUES ($1, $2, $3)")
        .bind(sender)
        .bind(recipient)
        .bind(Utc::now().naive_utc())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM dm_log WHERE sender = $1 AND sent_at <= $2")
        .bind(sender)
        .bind(window_start)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
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
    let cur_time: NaiveDateTime = Utc::now().naive_utc();

    for _ in 0..5 {
        let friend_code: i32 = random_range(100000..=999999);
        let upserted = sqlx::query_scalar::<_, bool>(
            "INSERT INTO hacker (hca_id, email, first_name, join_at, last_login_at, slack_id, friend_code) VALUES ($1, $2, $3, $4, $4, $5, $6)
            ON CONFLICT (hca_id) DO UPDATE SET last_login_at = EXCLUDED.last_login_at,
                slack_id = COALESCE(NULLIF(EXCLUDED.slack_id, ''), hacker.slack_id),
                email = COALESCE(NULLIF(EXCLUDED.email, ''), hacker.email),
                first_name = COALESCE(NULLIF(EXCLUDED.first_name, ''), hacker.first_name)
            RETURNING (xmax = 0)",
        )
        .bind(&auth_data.id)
        .bind(&auth_data.primary_email)
        .bind(&auth_data.first_name)
        .bind(cur_time)
        .bind(&auth_data.slack_id)
        .bind(friend_code)
        .fetch_one(pool)
        .await;
        match upserted {
            Err(e) if violates(&e, "hacker_friend_code_key") => continue,
            r => {
                if r? {
                    slack::dm(
                        &auth_data.slack_id,
                        "Thank you for signing up to Hack Club Dyad, looking forward to seeing what you create!".into(),
                    );
                }
                return Ok(());
            }
        }
    }
    bail!("Couldn't find an unused friend code")
}

pub enum LinkOutcome {
    Linked,
    Taken,
}

pub async fn on_link_hackatime(
    hackatime_token: String,
    req: &HttpRequest,
    pool: &Pool<Postgres>,
) -> anyhow::Result<LinkOutcome> {
    info!("Hackatime link started");
    let auth_data = auth::data::get_auth_data(req).await?;
    let hackatime_user = get_hackatime_user_with_token(&hackatime_token).await?;
    let hackatime_user_id = i64::try_from(hackatime_user.user_id)?;

    let mut tx = pool.begin().await?;
    let found: Option<(i32, Option<i64>)> = sqlx::query_as(
        "SELECT user_id, hackatime_user_id FROM hacker WHERE hca_id = $1 FOR UPDATE",
    )
    .bind(&auth_data.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((user_id, previous)) = found else {
        bail!("No Dyad account for this Hack Club login");
    };
    let encrypted = crypto::encrypt(user_id, &hackatime_token)?;

    sqlx::query(
        "INSERT INTO hackatime_account (hackatime_user_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(hackatime_user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    let owner: i32 =
        sqlx::query_scalar("SELECT user_id FROM hackatime_account WHERE hackatime_user_id = $1")
            .bind(hackatime_user_id)
            .fetch_one(&mut *tx)
            .await?;
    if owner != user_id {
        return Ok(LinkOutcome::Taken);
    }

    let linked =
        sqlx::query("UPDATE hacker SET hackatime_token = $1, hackatime_user_id = $2 WHERE user_id = $3")
            .bind(encrypted)
            .bind(hackatime_user_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await;
    match linked {
        Err(e) if violates(&e, "hacker_hackatime_user_id_unique") => {
            return Ok(LinkOutcome::Taken);
        }
        r => {
            r?;
        }
    }

    if previous != Some(hackatime_user_id) {
        let cleared = sqlx::query(
            "UPDATE user_project up SET hackatime_project_names = '{}' FROM project p
            WHERE p.project_id = up.project_id AND up.user_id = $1 AND p.shipped_at IS NULL
                AND up.airtable_record_id IS NULL AND (up.submitting_at IS NULL OR up.submitting_at <= $2)",
        )
        .bind(user_id)
        .bind(submit_cutoff())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if cleared > 0 {
            info!("Linked a different Hackatime account, so cleared the Hackatime projects on {cleared} projects");
        }
    }

    tx.commit().await?;
    Ok(LinkOutcome::Linked)
}
