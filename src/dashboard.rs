use actix_web::{HttpRequest, HttpResponse, cookie::Cookie, web};
use hc_helper::{
    airtable::AirtableTable,
    auth::{self, data::AuthData},
    submission::create_unified_with_token,
};
use log::warn;
use sqlx::{Pool, Postgres};

use crate::{
    db::{
        requests::{self, ProjectFields},
        schema::{
            ClientCatalog, ClientGame, ClientInvite, ClientMe, ClientMemberInfo, ClientProjectInfo,
            HackatimeTrackedProjectInfo, HackerParsed, ProjectParsed, UserProjectParsed,
        },
    },
    error::ApiError,
    games,
    hackatime::{get_hours_logged, get_summaries, round_hours},
    slack,
};

async fn current_hacker(
    req: &HttpRequest,
    pool: &Pool<Postgres>,
) -> Result<(HackerParsed, AuthData), ApiError> {
    let token = auth::login::get_auth_token_with_handling(req).ok_or(ApiError::Unauthorized)?;
    let auth_data = auth::data::get_auth_data_with_token(&token)
        .await
        .map_err(|e| {
            warn!("Auth data lookup failed: {e:?}");
            ApiError::Unauthorized
        })?;
    match requests::get_hacker(auth_data.id.clone(), pool).await {
        Ok(hacker) => Ok((hacker, auth_data)),
        Err(e) if matches!(e.downcast_ref(), Some(sqlx::Error::RowNotFound)) => {
            Err(ApiError::Unauthorized)
        }
        Err(e) => Err(e.into()),
    }
}

struct Partner {
    user_project: UserProjectParsed,
    hacker: HackerParsed,
}

struct Membership {
    project: ProjectParsed,
    mine: UserProjectParsed,
    friend: Option<Partner>,
}

async fn membership(
    project_id: i32,
    user_id: i32,
    pool: &Pool<Postgres>,
) -> Result<Membership, ApiError> {
    let mine = requests::get_user_project(project_id, user_id, pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let project = requests::get_project(project_id, pool).await?;
    let friend_id = if project.user1 == user_id {
        project.user2
    } else {
        Some(project.user1)
    };
    let friend = match friend_id {
        Some(friend_id) => Some(Partner {
            user_project: requests::get_user_project(project_id, friend_id, pool)
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("Project {project_id} is missing its second member")
                })?,
            hacker: requests::get_hacker_from_user_id(friend_id, pool).await?,
        }),
        None => None,
    };
    Ok(Membership {
        project,
        mine,
        friend,
    })
}

#[actix_web::get("/dashboard")]
pub async fn serve_dashboard(req: HttpRequest, pool: web::Data<Pool<Postgres>>) -> HttpResponse {
    if let Err(e) = current_hacker(&req, &pool).await {
        warn!("Not signed in ({e}), redirecting");
        return auth::login::get_login_redirect_response();
    }

    match std::fs::read_to_string("./ui/build/dashboard.html") {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[actix_web::get("/dashboard/me")]
pub async fn get_me(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, auth_data) = current_hacker(&req, &pool).await?;
    Ok(HttpResponse::Ok().json(ClientMe {
        first_name: hacker.first_name,
        friend_code: hacker.friend_code,
        hackatime_linked: !hacker.hackatime_token.is_empty(),
        ysws_eligible: auth_data.ysws_eligible,
        verification_status: auth_data.verification_status,
        reminders: !hacker.reminders_opt_out,
    }))
}

#[actix_web::get("/dashboard/games")]
pub async fn get_games() -> HttpResponse {
    let catalog = &*games::CATALOG;
    HttpResponse::Ok().json(ClientCatalog {
        credit_budget: catalog.credit_budget,
        games: catalog
            .games
            .iter()
            .map(|g| ClientGame {
                name: &g.name,
                credits: catalog.credits_for(g),
                copies_label: g.copies_label(),
                image_url: g.image_url(),
            })
            .collect(),
    })
}

#[actix_web::get("/dashboard/projects-list")]
pub async fn get_projects_list(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let mut out_projects: Vec<HackatimeTrackedProjectInfo> = get_summaries(&hacker.hackatime_token)
        .await
        .map_err(|e| ApiError::BadGateway(format!("Couldn't reach Hackatime: {e}")))?
        .into_iter()
        .map(|p| HackatimeTrackedProjectInfo {
            name: p.name,
            hours: round_hours(p.total_duration_seconds),
        })
        .collect();
    out_projects.sort_by(|a, b| b.hours.total_cmp(&a.hours));
    Ok(HttpResponse::Ok().json(out_projects))
}

async fn live_hours(hackatime_token: &str, user_project: &UserProjectParsed) -> f32 {
    get_hours_logged(hackatime_token, &user_project.hackatime_project_names)
        .await
        .unwrap_or_else(|e| {
            warn!("Couldn't get Hackatime hours: {e:?}");
            0.0
        })
}

fn member_info(
    hacker: &HackerParsed,
    user_project: &UserProjectParsed,
    hours_logged: f32,
) -> ClientMemberInfo {
    ClientMemberInfo {
        name: hacker.first_name.clone(),
        hours_logged,
        hours_needed: user_project.hours_needed,
        first_time: user_project.first_time,
        confirmed: user_project.confirmed_at.is_some(),
        ready_to_ship: user_project.ship_ok_at.is_some(),
        hackatime_linked: !hacker.hackatime_token.is_empty(),
    }
}

#[actix_web::get("/dashboard/projects")]
pub async fn get_project_card_data(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let project_ids = requests::get_users_user_project_ids(hacker.user_id, &pool).await?;

    let mut out = vec![];
    for pid in project_ids {
        let m = membership(pid, hacker.user_id, &pool).await?;
        let shipped = m.project.shipped_at.is_some();

        let my_hours = if shipped {
            m.mine.hours_logged
        } else {
            live_hours(&hacker.hackatime_token, &m.mine).await
        };
        let friend = match &m.friend {
            Some(f) => {
                let hours = if shipped {
                    f.user_project.hours_logged
                } else {
                    live_hours(&f.hacker.hackatime_token, &f.user_project).await
                };
                Some(member_info(&f.hacker, &f.user_project, hours))
            }
            None => None,
        };

        out.push(ClientProjectInfo {
            project_id: pid,
            name: m.project.name,
            description: m.project.description,
            screenshot_url: m.project.screenshot_url,
            code_url: m.project.code_url,
            playable_url: m.project.playable_url,
            games: m.project.games,
            shipped,
            review_status: m.project.review_status,
            hackatime_project_names: m.mine.hackatime_project_names.clone(),
            me: member_info(&hacker, &m.mine, my_hours),
            friend,
        })
    }

    Ok(HttpResponse::Ok().json(out))
}

#[derive(serde::Deserialize)]
struct UpsertProjectRequest {
    project_id: Option<i32>,
    friend_code: Option<i32>,
    name: String,
    description: String,
    screenshot_url: String,
    code_url: String,
    playable_url: String,
    games: Vec<String>,
    first_time: bool,
    hackatime_project_names: Vec<String>,
}

fn validate_url(label: &str, url: &str) -> Result<(), ApiError> {
    if url.is_empty()
        || ((url.starts_with("https://") || url.starts_with("http://")) && url.len() <= 500)
    {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "{label} must be a valid http(s) link"
        )))
    }
}

fn validate_fields(body: &UpsertProjectRequest) -> Result<ProjectFields, ApiError> {
    let fields = ProjectFields {
        name: body.name.trim().to_string(),
        description: body.description.trim().to_string(),
        screenshot_url: body.screenshot_url.trim().to_string(),
        code_url: body.code_url.trim().to_string(),
        playable_url: body.playable_url.trim().to_string(),
        games: games::CATALOG
            .games
            .iter()
            .filter(|g| body.games.contains(&g.name))
            .map(|g| g.name.clone())
            .collect(),
    };
    if fields.name.is_empty() || fields.name.len() > 100 {
        return Err(ApiError::BadRequest("Name must be 1-100 characters".into()));
    }
    if fields.description.len() > 2000 {
        return Err(ApiError::BadRequest("Description is too long".into()));
    }
    validate_url("Screenshot URL", &fields.screenshot_url)?;
    validate_url("Code URL", &fields.code_url)?;
    validate_url("Playable URL", &fields.playable_url)?;
    let catalog = &*games::CATALOG;
    if let Some(unknown) = body.games.iter().find(|n| catalog.find(n).is_none()) {
        return Err(ApiError::BadRequest(format!("Unknown game \"{unknown}\"")));
    }
    let credits: u8 = fields
        .games
        .iter()
        .filter_map(|n| catalog.find(n))
        .map(|g| catalog.credits_for(g))
        .sum();
    if credits > catalog.credit_budget {
        return Err(ApiError::BadRequest(format!(
            "Those games cost {credits} credits, but you only have {}",
            catalog.credit_budget
        )));
    }
    Ok(fields)
}

async fn validate_hackatime_names(
    hacker: &HackerParsed,
    project_id: i32,
    names: &[String],
    pool: &Pool<Postgres>,
) -> Result<Vec<String>, ApiError> {
    let mut names: Vec<String> = names.iter().map(|n| n.trim().to_string()).collect();
    names.sort();
    names.dedup();
    if names.is_empty() {
        return Ok(names);
    }

    let available = get_summaries(&hacker.hackatime_token)
        .await
        .map_err(|e| ApiError::BadGateway(format!("Couldn't reach Hackatime: {e}")))?;
    if let Some(missing) = names
        .iter()
        .find(|n| !available.iter().any(|p| &p.name == *n))
    {
        return Err(ApiError::BadRequest(format!(
            "\"{missing}\" isn't one of your Hackatime projects"
        )));
    }

    if requests::hackatime_names_in_use(hacker.user_id, project_id, &names, pool).await? {
        return Err(ApiError::BadRequest(
            "One of those Hackatime projects is already linked to another of your Dyad projects"
                .into(),
        ));
    }
    Ok(names)
}

#[actix_web::post("/dashboard/upsert-project")]
pub async fn upsert_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<UpsertProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let fields = validate_fields(&body)?;

    let project_id = match body.project_id {
        Some(project_id) => {
            let m = membership(project_id, hacker.user_id, &pool).await?;
            if m.mine.confirmed_at.is_none() {
                return Err(ApiError::BadRequest("Accept the invite first".into()));
            }
            if m.project.shipped_at.is_some() {
                return Err(ApiError::BadRequest(
                    "This project has already shipped".into(),
                ));
            }

            let project_fields_changed = fields.name != m.project.name
                || fields.description != m.project.description
                || fields.screenshot_url != m.project.screenshot_url
                || fields.code_url != m.project.code_url
                || fields.playable_url != m.project.playable_url
                || fields.games != m.project.games;
            let friend_ship_ok = m
                .friend
                .as_ref()
                .is_some_and(|f| f.user_project.ship_ok_at.is_some());
            if project_fields_changed && (m.mine.ship_ok_at.is_some() || friend_ship_ok) {
                return Err(ApiError::BadRequest(
                    "Project details are locked once either of you is ready to ship".into(),
                ));
            }

            let names =
                validate_hackatime_names(&hacker, project_id, &body.hackatime_project_names, &pool)
                    .await?;
            let my_fields_changed =
                names != m.mine.hackatime_project_names || body.first_time != m.mine.first_time;
            if my_fields_changed && m.mine.ship_ok_at.is_some() {
                return Err(ApiError::BadRequest(
                    "Your details are locked once you're ready to ship".into(),
                ));
            }

            requests::update_project_fields(project_id, &fields, &pool).await?;
            requests::update_user_project(
                project_id,
                hacker.user_id,
                body.first_time,
                &names,
                &pool,
            )
            .await?;
            project_id
        }
        None => {
            let friend = match body.friend_code {
                Some(friend_code) => {
                    let friend = requests::get_hacker_from_friend_code(friend_code, &pool)
                        .await?
                        .ok_or_else(|| {
                            ApiError::BadRequest("No one has that friend code".into())
                        })?;
                    if friend.user_id == hacker.user_id {
                        return Err(ApiError::BadRequest(
                            "That's your own friend code; enter your partner's".into(),
                        ));
                    }
                    Some(friend)
                }
                None => {
                    if requests::get_partnerless_project(hacker.user_id, &pool)
                        .await?
                        .is_some()
                    {
                        return Err(ApiError::BadRequest(
                            "You already have a project waiting for a partner".into(),
                        ));
                    }
                    None
                }
            };
            let names =
                validate_hackatime_names(&hacker, 0, &body.hackatime_project_names, &pool).await?;
            let project_id = requests::create_project(
                &fields,
                hacker.user_id,
                friend.as_ref().map(|f| f.user_id),
                body.first_time,
                &names,
                &pool,
            )
            .await?;
            if let Some(friend) = friend {
                slack::dm(
                    &friend.slack_id,
                    format!(
                        "{} invited you to build \"{}\" together on Dyad! Accept the invite on your dashboard.",
                        hacker.first_name, fields.name
                    ),
                );
            }
            project_id
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({ "project_id": project_id })))
}

#[derive(serde::Deserialize)]
struct RespondInviteRequest {
    project_id: i32,
    accept: bool,
    first_time: bool,
}

#[actix_web::post("/dashboard/respond-invite")]
pub async fn respond_invite(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<RespondInviteRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let m = membership(body.project_id, hacker.user_id, &pool).await?;
    if m.mine.confirmed_at.is_some() {
        return Err(ApiError::BadRequest(
            "You've already accepted this project".into(),
        ));
    }

    if body.accept {
        requests::confirm_user_project(body.project_id, hacker.user_id, body.first_time, &pool)
            .await?;
        if let Some(f) = &m.friend {
            slack::dm(
                &f.hacker.slack_id,
                format!(
                    "{} accepted your invite to \"{}\" on Dyad. Time to build!",
                    hacker.first_name, m.project.name
                ),
            );
        }
    } else {
        requests::delete_project(body.project_id, &pool).await?;
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct DeleteProjectRequest {
    project_id: i32,
}

#[actix_web::post("/dashboard/delete-project")]
pub async fn delete_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<DeleteProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let m = membership(body.project_id, hacker.user_id, &pool).await?;
    if m.friend.is_some() {
        return Err(ApiError::BadRequest(
            "Projects with a partner can't be deleted".into(),
        ));
    }
    requests::delete_project(body.project_id, &pool).await?;
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::get("/dashboard/invite/{friend_code}")]
pub async fn get_invite(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    path: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let sender = requests::get_hacker_from_friend_code(path.into_inner(), &pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest("That invite link isn't valid".into()))?;
    if sender.user_id == hacker.user_id {
        return Err(ApiError::BadRequest(
            "That's your own invite link; send it to your partner".into(),
        ));
    }
    let project = requests::get_partnerless_project(sender.user_id, &pool).await?;
    Ok(HttpResponse::Ok().json(ClientInvite {
        name: sender.first_name,
        project_name: project.map(|p| p.name),
    }))
}

#[derive(serde::Deserialize)]
struct JoinRequest {
    friend_code: i32,
    first_time: bool,
}

#[actix_web::post("/dashboard/join")]
pub async fn join_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<JoinRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let sender = requests::get_hacker_from_friend_code(body.friend_code, &pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest("That invite link isn't valid".into()))?;
    let project = requests::get_partnerless_project(sender.user_id, &pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest("That project already has a partner".into()))?;
    if !requests::join_project(project.project_id, hacker.user_id, body.first_time, &pool).await? {
        return Err(ApiError::BadRequest("Couldn't join that project".into()));
    }
    slack::dm(
        &sender.slack_id,
        format!(
            "{} joined \"{}\" with your invite link. Time to build!",
            hacker.first_name, project.name
        ),
    );
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct RemindersRequest {
    enabled: bool,
}

#[actix_web::post("/dashboard/reminders")]
pub async fn set_reminders(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<RemindersRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    requests::set_reminders_opt_out(hacker.user_id, !body.enabled, &pool).await?;
    Ok(HttpResponse::Ok().finish())
}

const JOIN_COOKIE: &str = "dyad_join";

#[actix_web::get("/join/{friend_code}")]
pub async fn invite_link(path: web::Path<i32>) -> HttpResponse {
    HttpResponse::Found()
        .cookie(
            Cookie::build(JOIN_COOKIE, path.into_inner().to_string())
                .path("/")
                .max_age(actix_web::cookie::time::Duration::days(1))
                .finish(),
        )
        .append_header(("Location", "/dashboard"))
        .finish()
}

#[derive(serde::Deserialize)]
struct ShipRequest {
    project_id: i32,
}

#[derive(serde::Serialize)]
struct DyadFields {
    #[serde(rename = "Games")]
    games: String,
    #[serde(rename = "Partner Email")]
    partner_email: String,
    #[serde(rename = "First Time")]
    first_time: bool,
    #[serde(rename = "Hours")]
    hours: f32,
}

async fn submit_member(
    table: &AirtableTable,
    project: &ProjectParsed,
    hacker: &HackerParsed,
    user_project: &UserProjectParsed,
    partner: &HackerParsed,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    if user_project.airtable_record_id.is_some() {
        return Ok(());
    }
    let record_id = create_unified_with_token(
        &hacker.hca_token,
        &hacker.hackatime_token,
        table.clone(),
        project.code_url.clone(),
        project.playable_url.clone(),
        project.screenshot_url.clone(),
        project.description.clone(),
        user_project.hackatime_project_names.clone(),
        DyadFields {
            games: project
                .games
                .iter()
                .map(|n| match games::CATALOG.find(n) {
                    Some(g) => format!("{n} ({})", g.copies_label()),
                    None => n.clone(),
                })
                .collect::<Vec<_>>()
                .join(", "),
            partner_email: partner.email.clone(),
            first_time: user_project.first_time,
            hours: user_project.hours_logged,
        },
    )
    .await?;
    requests::set_airtable_record_id(project.project_id, user_project.user_id, &record_id, pool)
        .await
}

#[actix_web::post("/dashboard/ship")]
pub async fn ship_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    table: web::Data<AirtableTable>,
    body: web::Json<ShipRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, auth_data) = current_hacker(&req, &pool).await?;
    let m = membership(body.project_id, hacker.user_id, &pool).await?;

    let Some(friend) = &m.friend else {
        return Err(ApiError::BadRequest(
            "You need a partner before shipping".into(),
        ));
    };
    if m.mine.confirmed_at.is_none() || friend.user_project.confirmed_at.is_none() {
        return Err(ApiError::BadRequest(
            "Both of you need to accept the project before shipping".into(),
        ));
    }
    if m.project.shipped_at.is_some() {
        return Err(ApiError::BadRequest(
            "This project has already shipped".into(),
        ));
    }

    if m.mine.ship_ok_at.is_none() {
        if !auth_data.ysws_eligible {
            return Err(ApiError::BadRequest(
                "You need to verify your identity with Hack Club before shipping".into(),
            ));
        }
        let p = &m.project;
        if [
            &p.name,
            &p.description,
            &p.code_url,
            &p.playable_url,
            &p.screenshot_url,
        ]
        .iter()
        .any(|f| f.is_empty())
            || p.games.is_empty()
        {
            return Err(ApiError::BadRequest(
                "Fill in the name, description, code URL, playable URL, screenshot and games before shipping"
                    .into(),
            ));
        }
        if m.mine.hackatime_project_names.is_empty() {
            return Err(ApiError::BadRequest(
                "Link at least one Hackatime project before shipping".into(),
            ));
        }
        let hours = get_hours_logged(&hacker.hackatime_token, &m.mine.hackatime_project_names)
            .await
            .map_err(|e| ApiError::BadGateway(format!("Couldn't reach Hackatime: {e}")))?;
        if hours < m.mine.hours_needed {
            return Err(ApiError::BadRequest(format!(
                "You've logged {hours}h of your {}h target",
                m.mine.hours_needed
            )));
        }
        requests::set_ship_ok(body.project_id, hacker.user_id, hours, &pool).await?;
        if friend.user_project.ship_ok_at.is_none() {
            slack::dm(
                &friend.hacker.slack_id,
                format!(
                    "{} is ready to ship \"{}\" and is waiting on you! Hit Ready to ship on your Dyad dashboard once you've hit your hours.",
                    hacker.first_name, m.project.name
                ),
            );
        }
    }

    let m = membership(body.project_id, hacker.user_id, &pool).await?;
    let Some(friend) = &m.friend else {
        return Err(ApiError::NotFound);
    };
    if friend.user_project.ship_ok_at.is_none() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({ "shipped": false })));
    }

    if table.token.is_empty() {
        return Err(anyhow::anyhow!("Airtable is not configured").into());
    }

    let mut me = hacker.clone();
    if let Some(token) = auth::login::get_auth_token_with_handling(&req) {
        me.hca_token = token;
    }
    async {
        submit_member(&table, &m.project, &me, &m.mine, &friend.hacker, &pool).await?;
        submit_member(&table, &m.project, &friend.hacker, &friend.user_project, &me, &pool).await
    }
    .await
    .map_err(|e: anyhow::Error| {
            log::error!("Submission failed for project {}: {e:?}", body.project_id);
            ApiError::BadGateway(
                "Couldn't submit for review. Try again; if it keeps failing, your partner may need to sign in to Dyad again.".into(),
            )
        })?;

    requests::set_shipped(body.project_id, &pool).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "shipped": true })))
}
