use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use actix_web::{
    HttpRequest, HttpResponse,
    cookie::{Cookie, SameSite},
    web,
};
use hc_helper::{
    airtable::AirtableTable,
    auth::{self, data::AuthData},
    submission::create_unified_with_token,
};
use log::{error, info, warn};
use sqlx::{Pool, Postgres};

use crate::{
    airtable,
    db::{
        requests::{
            self, ClaimOutcome, CreateOutcome, FriendLookup, JoinOutcome, MAX_PENDING_INVITES,
            ProjectFields, SaveOutcome, WithdrawOutcome,
        },
        schema::{
            ClientCatalog, ClientDashboard, ClientGame, ClientInvite, ClientJoinRequest, ClientMe,
            ClientMemberInfo, ClientOutgoingRequest, ClientProjectInfo,
            HackatimeTrackedProjectInfo, HackerParsed, ProjectParsed, UserProjectParsed,
        },
    },
    error::{ApiError, rejected_by},
    games,
    hackatime::{
        HackatimeStatus, get_summaries, get_summaries_for, hours_in, round_hours, seconds_in,
        status, unusable,
    },
    slack::{self, plain},
};

const HACKATIME_HOST: &str = "hackatime.hackclub.com";
const HCA_HOST: &str = "auth.hackclub.com";

const REJECTED_TTL: Duration = Duration::from_secs(60);
const REJECTED_MAX: usize = 10_000;

static REJECTED_TOKENS: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn recently_rejected(token: &str) -> bool {
    let mut rejected = REJECTED_TOKENS.lock().unwrap_or_else(PoisonError::into_inner);
    match rejected.get(token) {
        Some(at) if at.elapsed() < REJECTED_TTL => true,
        Some(_) => {
            rejected.remove(token);
            false
        }
        None => false,
    }
}

fn remember_rejected(token: String) {
    let mut rejected = REJECTED_TOKENS.lock().unwrap_or_else(PoisonError::into_inner);
    if rejected.len() >= REJECTED_MAX {
        rejected.retain(|_, at| at.elapsed() < REJECTED_TTL);
        if rejected.len() >= REJECTED_MAX {
            rejected.clear();
        }
    }
    rejected.insert(token, Instant::now());
}

async fn current_hacker(
    req: &HttpRequest,
    pool: &Pool<Postgres>,
) -> Result<(HackerParsed, AuthData), ApiError> {
    let token = auth::login::get_auth_token_with_handling(req).ok_or(ApiError::Unauthorized)?;
    if recently_rejected(&token) {
        return Err(ApiError::Unauthorized);
    }
    let auth_data = auth::data::get_auth_data_with_token(&token)
        .await
        .map_err(|e| {
            if rejected_by(&e).is_some() {
                remember_rejected(token.clone());
                return ApiError::Unauthorized;
            }
            warn!("Auth data lookup failed: {e:?}");
            ApiError::BadGateway("Couldn't reach Hack Club Auth. Try again in a bit.".into())
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

fn friend_id(project: &ProjectParsed, user_id: i32) -> Option<i32> {
    if project.user1 == user_id {
        project.user2
    } else {
        Some(project.user1)
    }
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
    let friend = match friend_id(&project, user_id) {
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
    if let Err(ApiError::Unauthorized) = current_hacker(&req, &pool).await {
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
        invite_token: hacker.invite_token,
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
        .map_err(HackatimeStatus::my_error)?
        .into_iter()
        .map(|p| HackatimeTrackedProjectInfo {
            unusable: unusable(&p).map(String::from),
            hours: round_hours(p.total_duration_seconds),
            name: p.name,
        })
        .collect();
    out_projects.sort_by(|a, b| b.hours.total_cmp(&a.hours));
    Ok(HttpResponse::Ok().json(out_projects))
}

fn token_status(hacker: &HackerParsed) -> HackatimeStatus {
    if hacker.hackatime_token.is_empty() {
        HackatimeStatus::Unlinked
    } else {
        HackatimeStatus::Ok
    }
}

#[actix_web::get("/dashboard/projects")]
pub async fn get_project_card_data(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let projects = requests::get_projects_for_user(hacker.user_id, &pool).await?;
    let project_ids: Vec<i32> = projects.iter().map(|p| p.project_id).collect();
    let user_projects = requests::get_user_projects(&project_ids, &pool).await?;
    let friend_ids: Vec<i32> = projects
        .iter()
        .filter_map(|p| friend_id(p, hacker.user_id))
        .collect();
    let friends: HashMap<i32, HackerParsed> = requests::get_hackers(&friend_ids, &pool)
        .await?
        .into_iter()
        .map(|h| (h.user_id, h))
        .collect();
    let mut incoming: HashMap<i32, Vec<ClientJoinRequest>> = HashMap::new();
    for row in requests::get_incoming_join_requests(hacker.user_id, &pool).await? {
        incoming.entry(row.project_id).or_default().push(ClientJoinRequest {
            user_id: row.user_id,
            name: row.name,
        });
    }
    let outgoing = requests::get_outgoing_join_requests(hacker.user_id, &pool)
        .await?
        .into_iter()
        .map(|r| ClientOutgoingRequest {
            project_id: r.project_id,
            project_name: r.project_name,
            owner_name: r.owner_name,
        })
        .collect();

    let user_project = |project_id: i32, user_id: i32| {
        user_projects
            .iter()
            .find(|up| up.project_id == project_id && up.user_id == user_id)
    };
    let mut tokens = HashSet::from([hacker.hackatime_token.clone()]);
    for p in projects.iter().filter(|p| p.shipped_at.is_none()) {
        let Some(friend) = friend_id(p, hacker.user_id).and_then(|id| friends.get(&id)) else {
            continue;
        };
        let both_confirmed = [hacker.user_id, friend.user_id]
            .iter()
            .all(|&id| user_project(p.project_id, id).is_some_and(|up| up.confirmed_at.is_some()));
        if both_confirmed {
            tokens.insert(friend.hackatime_token.clone());
        }
    }
    let summaries = get_summaries_for(tokens).await;

    let member = |h: &HackerParsed, up: &UserProjectParsed, shipped: bool| {
        let (hours_logged, hackatime) = if shipped {
            (Some(up.hours_logged), token_status(h))
        } else {
            match summaries.get(&h.hackatime_token) {
                Some(Ok(projects)) => (
                    Some(hours_in(projects, &up.hackatime_project_names)),
                    HackatimeStatus::Ok,
                ),
                Some(Err(status)) => (None, *status),
                None => (None, token_status(h)),
            }
        };
        ClientMemberInfo {
            user_id: h.user_id,
            name: h.first_name.clone(),
            hours_logged,
            hours_needed: up.hours_needed,
            first_time: up.first_time,
            confirmed: up.confirmed_at.is_some(),
            ready_to_ship: up.ship_ok_at.is_some(),
            submitting: !shipped && requests::is_submitting(up.submitting_at),
            hackatime,
        }
    };

    let mut out = vec![];
    for p in projects {
        let Some(mine) = user_project(p.project_id, hacker.user_id) else {
            continue;
        };
        let shipped = p.shipped_at.is_some();
        let friend = friend_id(&p, hacker.user_id)
            .and_then(|id| Some((friends.get(&id)?, user_project(p.project_id, id)?)))
            .map(|(h, up)| member(h, up, shipped));
        out.push(ClientProjectInfo {
            project_id: p.project_id,
            name: p.name,
            description: p.description,
            screenshot_url: p.screenshot_url,
            code_url: p.code_url,
            playable_url: p.playable_url,
            games: p.games,
            shipped,
            review_status: p.review_status,
            hackatime_project_names: mine.hackatime_project_names.clone(),
            me: member(&hacker, mine, shipped),
            friend,
            join_requests: incoming.remove(&p.project_id).unwrap_or_default(),
        })
    }

    Ok(HttpResponse::Ok().json(ClientDashboard {
        hackatime: summaries
            .get(&hacker.hackatime_token)
            .map(status)
            .unwrap_or_else(|| token_status(&hacker)),
        projects: out,
        join_requests: outgoing,
    }))
}

#[derive(serde::Deserialize)]
struct UpsertProjectRequest {
    project_id: Option<i32>,
    friend_code: Option<i32>,
    invite_token: Option<String>,
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
    if url.is_empty() {
        return Ok(());
    }
    let valid = url.chars().count() <= 500
        && reqwest::Url::parse(url).is_ok_and(|u| {
            matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty())
        });
    if valid {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "{label} must be a valid http(s) link"
        )))
    }
}

fn validate_fields(
    body: &UpsertProjectRequest,
    previous_games: &[String],
) -> Result<ProjectFields, ApiError> {
    let catalog = &*games::CATALOG;
    let retired = |n: &&String| catalog.find(n).is_none() && previous_games.contains(n);
    let fields = ProjectFields {
        name: body.name.trim().to_string(),
        description: body.description.trim().to_string(),
        screenshot_url: body.screenshot_url.trim().to_string(),
        code_url: body.code_url.trim().to_string(),
        playable_url: body.playable_url.trim().to_string(),
        games: catalog
            .games
            .iter()
            .filter(|g| body.games.contains(&g.name))
            .map(|g| g.name.clone())
            .chain(body.games.iter().filter(retired).cloned())
            .collect(),
    };
    let name_length = fields.name.chars().count();
    if name_length == 0 || name_length > 100 {
        return Err(ApiError::BadRequest("Name must be 1-100 characters".into()));
    }
    if fields.description.chars().count() > 2000 {
        return Err(ApiError::BadRequest("Description is too long".into()));
    }
    validate_url("Screenshot URL", &fields.screenshot_url)?;
    validate_url("Code URL", &fields.code_url)?;
    validate_url("Playable URL", &fields.playable_url)?;
    if let Some(unknown) = body
        .games
        .iter()
        .find(|n| catalog.find(n).is_none() && !previous_games.contains(n))
    {
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
    names: &[String],
    previous: &[String],
) -> Result<Vec<String>, ApiError> {
    let mut names: Vec<String> = names.iter().map(|n| n.trim().to_string()).collect();
    names.sort();
    names.dedup();
    let added: Vec<&String> = names.iter().filter(|n| !previous.contains(n)).collect();
    if added.is_empty() {
        return Ok(names);
    }

    let available = get_summaries(&hacker.hackatime_token)
        .await
        .map_err(HackatimeStatus::my_error)?;
    for name in added {
        let Some(project) = available.iter().find(|p| &p.name == name) else {
            return Err(ApiError::BadRequest(format!(
                "\"{name}\" isn't one of your Hackatime projects"
            )));
        };
        if let Some(reason) = unusable(project) {
            return Err(ApiError::BadRequest(format!("Can't use \"{name}\". {reason}")));
        }
    }
    Ok(names)
}

fn too_many_invites() -> ApiError {
    ApiError::BadRequest(format!(
        "You already have {MAX_PENDING_INVITES} invites waiting for a reply. Wait for one to be answered or delete one first."
    ))
}

async fn dm_limited(pool: &Pool<Postgres>, sender: i32, recipient: &HackerParsed, text: String) {
    if !slack::enabled() || recipient.slack_id.is_empty() {
        return;
    }
    match requests::allow_dm(sender, recipient.user_id, pool).await {
        Ok(true) => slack::dm(&recipient.slack_id, text),
        Ok(false) => {}
        Err(e) => warn!(
            "Couldn't check the DM limit from user {sender} to user {}: {e:?}",
            recipient.user_id
        ),
    }
}

fn names_in_use() -> ApiError {
    ApiError::BadRequest(
        "One of those Hackatime projects is already linked to another of your Dyad projects"
            .into(),
    )
}

#[actix_web::post("/dashboard/upsert-project")]
pub async fn upsert_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<UpsertProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;

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

            let fields = validate_fields(&body, &m.project.games)?;
            let project_fields_changed = fields.name != m.project.name
                || fields.description != m.project.description
                || fields.screenshot_url != m.project.screenshot_url
                || fields.code_url != m.project.code_url
                || fields.playable_url != m.project.playable_url
                || fields.games.iter().collect::<HashSet<_>>()
                    != m.project.games.iter().collect::<HashSet<_>>();
            let locked = |up: &UserProjectParsed| {
                up.ship_ok_at.is_some() || requests::is_submitting(up.submitting_at)
            };
            let friend_locked = m.friend.as_ref().is_some_and(|f| locked(&f.user_project));
            if project_fields_changed && (locked(&m.mine) || friend_locked) {
                return Err(ApiError::BadRequest(
                    "Project details are locked once either of you is ready to ship".into(),
                ));
            }

            let names = validate_hackatime_names(
                &hacker,
                &body.hackatime_project_names,
                &m.mine.hackatime_project_names,
            )
            .await?;
            let my_fields_changed =
                names != m.mine.hackatime_project_names || body.first_time != m.mine.first_time;
            if my_fields_changed && locked(&m.mine) {
                return Err(ApiError::BadRequest(
                    "Your details are locked once you're ready to ship".into(),
                ));
            }

            if project_fields_changed || my_fields_changed {
                match requests::save_project(
                    project_id,
                    hacker.user_id,
                    project_fields_changed.then_some(&fields),
                    my_fields_changed.then_some((body.first_time, names.as_slice())),
                    &pool,
                )
                .await?
                {
                    SaveOutcome::Saved => {}
                    SaveOutcome::NamesInUse => return Err(names_in_use()),
                    SaveOutcome::Locked => {
                        return Err(ApiError::BadRequest(
                            "This project changed while you were editing. Refresh and try again."
                                .into(),
                        ));
                    }
                }
            }
            project_id
        }
        None => {
            let fields = validate_fields(&body, &[])?;
            let friend = match (&body.invite_token, body.friend_code) {
                (Some(invite_token), _) => Some(invite_sender(invite_token, &hacker, &pool).await?),
                (None, Some(friend_code)) => {
                    if requests::has_too_many_invites(hacker.user_id, &pool).await? {
                        return Err(too_many_invites());
                    }
                    let friend = match requests::find_friend(hacker.user_id, friend_code, &pool)
                        .await?
                    {
                        FriendLookup::Found(friend) => friend,
                        FriendLookup::NotFound => {
                            return Err(ApiError::BadRequest(
                                "No one has that friend code".into(),
                            ));
                        }
                        FriendLookup::TooManyAttempts => {
                            return Err(ApiError::BadRequest(
                                "Too many wrong friend codes. Try again later.".into(),
                            ));
                        }
                    };
                    if friend.user_id == hacker.user_id {
                        return Err(ApiError::BadRequest(
                            "That's your own friend code, enter your partner's ya goober!".into(),
                        ));
                    }
                    Some(friend)
                }
                (None, None) => None,
            };
            let names =
                validate_hackatime_names(&hacker, &body.hackatime_project_names, &[]).await?;
            let project_id = match requests::create_project(
                &fields,
                hacker.user_id,
                friend.as_ref().map(|f| f.user_id),
                body.first_time,
                &names,
                &pool,
            )
            .await?
            {
                CreateOutcome::Created(project_id) => project_id,
                CreateOutcome::NamesInUse => return Err(names_in_use()),
                CreateOutcome::PartnerlessTaken => {
                    return Err(ApiError::BadRequest(
                        "You already have a project waiting for a partner".into(),
                    ));
                }
                CreateOutcome::TooManyInvites => return Err(too_many_invites()),
            };
            if let Some(friend) = friend {
                dm_limited(
                    &pool,
                    hacker.user_id,
                    &friend,
                    format!(
                        "{} invited you to build \"{}\" together on Dyad! Accept the invite on your dashboard.",
                        plain(&hacker.first_name),
                        plain(&fields.name)
                    ),
                )
                .await;
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
    let name = plain(&hacker.first_name);
    let project_name = plain(&m.project.name);

    if body.accept {
        if !requests::confirm_user_project(body.project_id, hacker.user_id, body.first_time, &pool)
            .await?
        {
            return Err(ApiError::BadRequest("This invite was withdrawn".into()));
        }
        if let Some(f) = &m.friend {
            dm_limited(
                &pool,
                hacker.user_id,
                &f.hacker,
                format!("{name} accepted your invite to \"{project_name}\" on Dyad. Time to build!"),
            )
            .await;
        }
    } else {
        let Some(kept) = requests::decline_invite(body.project_id, hacker.user_id, &pool).await?
        else {
            return Err(ApiError::BadRequest(
                "This invite is no longer pending".into(),
            ));
        };
        if let Some(f) = &m.friend {
            let message = if kept {
                format!(
                    "{name} declined your invite to \"{project_name}\" on Dyad. You can invite someone new if you want."
                )
            } else {
                format!(
                    "{name} declined your invite to \"{project_name}\" on Dyad. It was removed because you already have another project waiting for a partner."
                )
            };
            dm_limited(&pool, hacker.user_id, &f.hacker, message).await;
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct ProjectRequest {
    project_id: i32,
}

#[actix_web::post("/dashboard/delete-project")]
pub async fn delete_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<ProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let Some(deleted) = requests::delete_project(body.project_id, hacker.user_id, &pool).await?
    else {
        return Err(ApiError::BadRequest(
            "Only the creator can delete a project, and only before a partner accepts".into(),
        ));
    };

    let name = plain(&hacker.first_name);
    let project_name = plain(&deleted.name);
    let notify: Vec<i32> = deleted
        .invitee
        .into_iter()
        .chain(deleted.requesters.iter().copied())
        .collect();
    for h in requests::get_hackers(&notify, &pool).await? {
        let message = if Some(h.user_id) == deleted.invitee {
            format!("{name} deleted \"{project_name}\" on Dyad, so their invite to you was withdrawn.")
        } else {
            format!("{name} deleted \"{project_name}\" on Dyad, so your request to join was closed.")
        };
        dm_limited(&pool, hacker.user_id, &h, message).await;
    }
    Ok(HttpResponse::Ok().finish())
}

async fn invite_sender(
    invite_token: &str,
    hacker: &HackerParsed,
    pool: &Pool<Postgres>,
) -> Result<HackerParsed, ApiError> {
    let sender = requests::get_hacker_from_invite_token(invite_token, pool)
        .await?
        .ok_or_else(|| ApiError::BadRequest("That invite link isn't valid".into()))?;
    if sender.user_id == hacker.user_id {
        return Err(ApiError::BadRequest(
            "That's your own invite link; send it to your partner".into(),
        ));
    }
    Ok(sender)
}

#[actix_web::get("/dashboard/invite/{invite_token}")]
pub async fn get_invite(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let sender = invite_sender(&path, &hacker, &pool).await?;
    let project = requests::get_partnerless_project(sender.user_id, &pool).await?;
    Ok(HttpResponse::Ok().json(ClientInvite {
        name: sender.first_name,
        project_name: project.map(|p| p.name),
    }))
}

#[derive(serde::Deserialize)]
struct JoinRequest {
    invite_token: String,
    first_time: bool,
}

#[actix_web::post("/dashboard/join")]
pub async fn join_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<JoinRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let sender = invite_sender(&body.invite_token, &hacker, &pool).await?;
    let project = requests::get_partnerless_project(sender.user_id, &pool)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "{} doesn't have a project waiting for a partner right now",
                sender.first_name
            ))
        })?;
    match requests::request_join(project.project_id, hacker.user_id, body.first_time, &pool)
        .await?
    {
        JoinOutcome::Requested => {}
        JoinOutcome::AlreadyRequested => {
            return Err(ApiError::BadRequest(
                "You already asked to join that project".into(),
            ));
        }
        JoinOutcome::Unavailable => {
            return Err(ApiError::BadRequest(
                "That project already has a partner".into(),
            ));
        }
    }
    dm_limited(
        &pool,
        hacker.user_id,
        &sender,
        format!(
            "{} asked to join \"{}\" on Dyad. Approve or decline on your dashboard.",
            plain(&hacker.first_name),
            plain(&project.name)
        ),
    )
    .await;
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct RespondJoinRequest {
    project_id: i32,
    user_id: i32,
    approve: bool,
}

#[actix_web::post("/dashboard/respond-join")]
pub async fn respond_join(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<RespondJoinRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let project = requests::get_partnerless_project(hacker.user_id, &pool)
        .await?
        .filter(|p| p.project_id == body.project_id)
        .ok_or_else(|| ApiError::BadRequest("That request is no longer open".into()))?;
    let name = plain(&hacker.first_name);
    let project_name = plain(&project.name);

    if body.approve {
        let Some(others) =
            requests::approve_join(body.project_id, hacker.user_id, body.user_id, &pool).await?
        else {
            return Err(ApiError::BadRequest(
                "That request is no longer open".into(),
            ));
        };
        let requester = requests::get_hacker_from_user_id(body.user_id, &pool).await?;
        dm_limited(
            &pool,
            hacker.user_id,
            &requester,
            format!("{name} approved your request to join \"{project_name}\" on Dyad. Time to get building!"),
        )
        .await;
        for h in requests::get_hackers(&others, &pool).await? {
            dm_limited(
                &pool,
                hacker.user_id,
                &h,
                format!("{name} picked another partner for \"{project_name}\" on Dyad, so your request to join was closed."),
            )
            .await;
        }
    } else {
        if !requests::decline_join(body.project_id, hacker.user_id, body.user_id, &pool).await? {
            return Err(ApiError::BadRequest(
                "That request is no longer open".into(),
            ));
        }
        let requester = requests::get_hacker_from_user_id(body.user_id, &pool).await?;
        dm_limited(
            &pool,
            hacker.user_id,
            &requester,
            format!("{name} declined your request to join \"{project_name}\" on Dyad."),
        )
        .await;
    }
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::post("/dashboard/cancel-join")]
pub async fn cancel_join(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    body: web::Json<ProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    requests::cancel_join(body.project_id, hacker.user_id, &pool).await?;
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::post("/dashboard/regenerate-invite")]
pub async fn regenerate_invite(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    _: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let invite_token = requests::regenerate_invite_token(hacker.user_id, &pool).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "invite_token": invite_token })))
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

#[actix_web::get("/join/{invite_token}")]
pub async fn invite_link(path: web::Path<String>) -> HttpResponse {
    let invite_token = path.into_inner();
    let mut response = HttpResponse::Found();
    response.append_header(("Location", "/dashboard"));
    if invite_token.len() == 32 && invite_token.bytes().all(|b| b.is_ascii_hexdigit()) {
        response.cookie(
            Cookie::build(JOIN_COOKIE, invite_token)
                .path("/")
                .max_age(actix_web::cookie::time::Duration::days(1))
                .same_site(SameSite::Lax)
                .secure(true)
                .finish(),
        );
    }
    response.finish()
}

#[actix_web::post("/dashboard/unready")]
pub async fn unready(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    table: web::Data<AirtableTable>,
    body: web::Json<ProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let (hacker, _) = current_hacker(&req, &pool).await?;
    let m = membership(body.project_id, hacker.user_id, &pool).await?;
    tokio::spawn(withdraw(
        table.get_ref().clone(),
        pool.get_ref().clone(),
        body.project_id,
        hacker.user_id,
        m.mine.hours_logged,
    ))
    .await
    .map_err(anyhow::Error::from)??;
    Ok(HttpResponse::Ok().finish())
}

async fn withdraw(
    table: AirtableTable,
    pool: Pool<Postgres>,
    project_id: i32,
    user_id: i32,
    hours_logged: f32,
) -> Result<(), ApiError> {
    let (record_id, claim) = match requests::withdraw_submission(project_id, user_id, &pool).await?
    {
        WithdrawOutcome::NotReady | WithdrawOutcome::Withdrawn(None) => {
            return Ok(());
        }
        WithdrawOutcome::Shipped => {
            return Err(ApiError::BadRequest(
                "This project has already shipped".into(),
            ));
        }
        WithdrawOutcome::Busy => {
            return Err(ApiError::BadRequest(
                "Your submission is still in progress. Give it a minute and refresh.".into(),
            ));
        }
        WithdrawOutcome::Withdrawn(Some(withdrawn)) => withdrawn,
    };

    let deleted = match airtable::delete_record(&table, &record_id).await {
        Ok(()) => true,
        Err(e) => {
            warn!(
                "Couldn't delete Airtable record {record_id} for project {project_id}: {}",
                airtable::describe(&e)
            );
            match airtable::record_exists(&table, &record_id).await {
                Ok(exists) => !exists,
                Err(e) => {
                    warn!(
                        "Couldn't check Airtable record {record_id} for project {project_id}: {}",
                        airtable::describe(&e)
                    );
                    false
                }
            }
        }
    };
    if deleted {
        requests::release_submission(project_id, user_id, claim, &pool).await?;
        return Ok(());
    }

    match requests::finish_submission(project_id, user_id, claim, &record_id, hours_logged, &pool)
        .await
    {
        Ok(Some(recorded)) if recorded.shipped => {
            mark_pair_complete(&table, &recorded.record_ids).await
        }
        Ok(Some(_)) => {}
        Ok(None) => error!(
            "Couldn't restore Airtable record {record_id} on project {project_id} because it changed meanwhile; delete the record in Airtable"
        ),
        Err(e) => {
            error!("Couldn't restore Airtable record {record_id} on project {project_id}: {e:?}")
        }
    }
    Err(ApiError::BadGateway(
        "Couldn't reach Airtable to withdraw your submission. Try again in a bit.".into(),
    ))
}

async fn mark_pair_complete(table: &AirtableTable, record_ids: &[String]) {
    if let Err(e) = airtable::set_pair_complete(table, record_ids, true).await {
        warn!(
            "Couldn't mark {record_ids:?} as a complete pair: {}",
            airtable::describe(&e)
        );
    }
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

fn failed_hackatime_project(e: &anyhow::Error) -> Option<String> {
    let missing = e
        .chain()
        .filter_map(|c| c.downcast_ref::<reqwest::Error>())
        .any(|e| e.status().is_some_and(|s| s.as_u16() == 404));
    if !missing {
        return None;
    }
    e.chain().find_map(|c| {
        c.to_string()
            .strip_prefix("Failed to get hackatime project: ")
            .map(String::from)
    })
}

fn submission_error(project_id: i32, e: anyhow::Error) -> ApiError {
    error!(
        "Submission failed for project {project_id}: {}",
        airtable::describe(&e)
    );
    let text = format!("{e:#}");
    let message = match rejected_by(&e).as_deref() {
        Some(HCA_HOST) => "Your Hack Club sign-in expired. Sign in to Dyad again, then try again.".into(),
        Some(HACKATIME_HOST) => {
            "Your Hackatime connection stopped working. Relink it from your dashboard, then try again.".into()
        }
        _ if text.contains("not YSWS-eligible") => {
            "You need to verify your identity with Hack Club before shipping".into()
        }
        _ if text.contains("No primary address") => {
            "Add a mailing address at auth.hackclub.com, then try again.".into()
        }
        _ => match failed_hackatime_project(&e) {
            Some(name) => format!(
                "Hackatime can't find your project \"{name}\". If you renamed or deleted it, edit this project to update your Hackatime projects, then try again."
            ),
            None if text.contains("heartbeat") => {
                "One of your Hackatime projects has no time logged. Edit this project to update your Hackatime projects, then try again.".into()
            }
            None => {
                return ApiError::BadGateway(
                    "Couldn't submit for review. Try again in a bit.".into(),
                );
            }
        },
    };
    ApiError::BadRequest(message)
}

fn shipped_response(shipped: bool) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "shipped": shipped }))
}

async fn check_ready(
    project_id: i32,
    hacker: &HackerParsed,
    auth_data: &AuthData,
    pool: &Pool<Postgres>,
) -> Result<(Membership, f32), ApiError> {
    if !auth_data.ysws_eligible {
        return Err(ApiError::BadRequest(
            "You need to verify your identity with Hack Club before shipping".into(),
        ));
    }
    if auth_data.addresses.is_empty() {
        return Err(ApiError::BadRequest(
            "Add a mailing address at auth.hackclub.com before shipping".into(),
        ));
    }
    let m = membership(project_id, hacker.user_id, pool).await?;
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
    let projects = get_summaries(&hacker.hackatime_token)
        .await
        .map_err(HackatimeStatus::my_error)?;
    for name in &m.mine.hackatime_project_names {
        let project = projects.iter().find(|p| &p.name == name);
        if let Some(reason) = project.and_then(unusable) {
            return Err(ApiError::BadRequest(format!(
                "Can't ship with \"{name}\". {reason}"
            )));
        }
        if !project.is_some_and(|p| p.total_duration_seconds > 0.0) {
            return Err(ApiError::BadRequest(format!(
                "Your Hackatime project \"{name}\" has no time logged. If you renamed or deleted it, edit this project to update your Hackatime projects."
            )));
        }
    }
    let seconds = seconds_in(&projects, &m.mine.hackatime_project_names);
    if seconds < f64::from(m.mine.hours_needed) * 3600.0 {
        return Err(ApiError::BadRequest(format!(
            "You've logged {}h of your {}h target",
            (seconds / 36.0).floor() / 100.0,
            m.mine.hours_needed
        )));
    }
    let hours = hours_in(&projects, &m.mine.hackatime_project_names);
    Ok((m, hours))
}

struct Submission {
    table: AirtableTable,
    pool: Pool<Postgres>,
    hca_token: String,
    hacker: HackerParsed,
    project: ProjectParsed,
    mine: UserProjectParsed,
    friend: HackerParsed,
    claim: sqlx::types::chrono::NaiveDateTime,
    hours: f32,
}

async fn release(s: &Submission) {
    if let Err(e) =
        requests::release_submission(s.project.project_id, s.hacker.user_id, s.claim, &s.pool).await
    {
        warn!(
            "Couldn't release the submission claim on project {}: {e:?}",
            s.project.project_id
        );
    }
}

async fn submit(s: Submission) -> Result<bool, ApiError> {
    let project_id = s.project.project_id;
    let created = create_unified_with_token(
        &s.hca_token,
        &s.hacker.hackatime_token,
        s.table.clone(),
        s.project.code_url.clone(),
        s.project.playable_url.clone(),
        s.project.screenshot_url.clone(),
        s.project.description.clone(),
        s.mine.hackatime_project_names.clone(),
        DyadFields {
            games: s
                .project
                .games
                .iter()
                .map(|n| match games::CATALOG.find(n) {
                    Some(g) => format!("{n} ({})", g.copies_label()),
                    None => n.clone(),
                })
                .collect::<Vec<_>>()
                .join(", "),
            partner_email: s.friend.email.clone(),
            first_time: s.mine.first_time,
            hours: s.hours,
        },
    )
    .await;
    let record_id = match created {
        Ok(record_id) => record_id,
        Err(e) => {
            release(&s).await;
            return Err(submission_error(project_id, e));
        }
    };

    let finished = requests::finish_submission(
        project_id,
        s.hacker.user_id,
        s.claim,
        &record_id,
        s.hours,
        &s.pool,
    )
    .await;
    let recorded = match finished {
        Ok(Some(recorded)) => recorded,
        stale_or_failed => {
            error!(
                "Couldn't save Airtable record {record_id} for project {project_id}, so deleting it: {:?}",
                stale_or_failed.as_ref().err()
            );
            if let Err(e) = airtable::delete_record(&s.table, &record_id).await {
                error!(
                    "Couldn't delete orphaned Airtable record {record_id}: {}",
                    airtable::describe(&e)
                );
            }
            release(&s).await;
            return Err(ApiError::BadGateway(
                "Couldn't submit for review. Try again in a bit.".into(),
            ));
        }
    };

    info!("Submitted Airtable record {record_id} for project {project_id}");
    if recorded.shipped {
        mark_pair_complete(&s.table, &recorded.record_ids).await;
    } else {
        dm_limited(
            &s.pool,
            s.hacker.user_id,
            &s.friend,
            format!(
                "{} is ready to ship \"{}\" and is waiting on you! Hit Ready to ship on your Dyad dashboard once you've hit your hours.",
                plain(&s.hacker.first_name),
                plain(&s.project.name)
            ),
        )
        .await;
    }
    Ok(recorded.shipped)
}

const SHIP_WAIT: Duration = Duration::from_secs(30);

#[actix_web::post("/dashboard/ship")]
pub async fn ship_project(
    req: HttpRequest,
    pool: web::Data<Pool<Postgres>>,
    table: web::Data<AirtableTable>,
    body: web::Json<ProjectRequest>,
) -> Result<HttpResponse, ApiError> {
    let hca_token = auth::login::get_auth_token_with_handling(&req).ok_or(ApiError::Unauthorized)?;
    auth::data::forget_auth_token(&hca_token);
    let (hacker, auth_data) = current_hacker(&req, &pool).await?;
    let project_id = body.project_id;
    let m = membership(project_id, hacker.user_id, &pool).await?;

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
        return Ok(shipped_response(true));
    }
    if m.mine.airtable_record_id.is_some() {
        return Ok(shipped_response(false));
    }
    if !airtable::configured(&table) || !airtable::ready() {
        error!("Airtable isn't set up, so project {project_id} can't be submitted");
        return Err(ApiError::BadGateway(
            "Submitting isn't available right now. Try again later.".into(),
        ));
    }

    let claim = match requests::claim_submission(project_id, hacker.user_id, &pool).await? {
        ClaimOutcome::Claimed(claim) => claim,
        ClaimOutcome::Shipped => return Ok(shipped_response(true)),
        ClaimOutcome::AlreadyReady => return Ok(shipped_response(false)),
        ClaimOutcome::Busy => {
            return Err(ApiError::BadRequest(
                "Your submission is already in progress. Give it a minute and refresh.".into(),
            ));
        }
        ClaimOutcome::FirstTimeUsed => {
            return Err(ApiError::BadRequest(
                "You've already shipped a first time project. Edit this one and uncheck first time (it needs 5h instead of 3h), then try again."
                    .into(),
            ));
        }
    };

    let ready = check_ready(project_id, &hacker, &auth_data, &pool)
        .await
        .and_then(|(m, hours)| match m.friend {
            Some(friend) => Ok((m.project, m.mine, friend.hacker, hours)),
            None => Err(ApiError::NotFound),
        });
    let (project, mine, friend, hours) = match ready {
        Ok(ready) => ready,
        Err(e) => {
            if let Err(e) =
                requests::release_submission(project_id, hacker.user_id, claim, &pool).await
            {
                warn!("Couldn't release the submission claim on project {project_id}: {e:?}");
            }
            return Err(e);
        }
    };

    let task = tokio::spawn(submit(Submission {
        table: table.get_ref().clone(),
        pool: pool.get_ref().clone(),
        hca_token,
        hacker,
        project,
        mine,
        friend,
        claim,
        hours,
    }));
    match tokio::time::timeout(SHIP_WAIT, task).await {
        Ok(Ok(shipped)) => Ok(shipped_response(shipped?)),
        Ok(Err(e)) => Err(anyhow::Error::from(e).into()),
        Err(_) => Ok(HttpResponse::Ok().json(serde_json::json!({ "shipped": false, "submitting": true }))),
    }
}
