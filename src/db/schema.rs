use sqlx::types::chrono::NaiveDateTime;

use crate::{crypto, hackatime::HackatimeStatus};

#[derive(sqlx::FromRow)]
pub struct DBHacker {
    pub hca_id: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub hackatime_token: Option<String>,
    pub user_id: Option<i32>,
    pub join_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub slack_id: Option<String>,
    pub friend_code: Option<i32>,
    pub reminders_opt_out: Option<bool>,
    pub dms_opt_out: Option<bool>,
    pub invite_token: Option<String>,
    pub hackatime_user_id: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct DBProject {
    pub project_id: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub screenshot_url: Option<String>,
    pub code_url: Option<String>,
    pub playable_url: Option<String>,
    pub games: Option<Vec<String>>,
    pub user1: Option<i32>,
    pub user2: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub shipped_at: Option<NaiveDateTime>,
    pub approved_at: Option<NaiveDateTime>,
    pub review_status: Option<String>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct DBUserProject {
    pub user_id: Option<i32>,
    pub project_id: Option<i32>,
    pub first_time: Option<bool>,
    pub hours_needed: Option<f32>,
    pub hours_logged: Option<f32>,
    pub confirmed_at: Option<NaiveDateTime>,
    pub ship_ok_at: Option<NaiveDateTime>,
    pub hackatime_project_names: Option<Vec<String>>,
    pub airtable_record_id: Option<String>,
    pub nudged_at: Option<NaiveDateTime>,
    pub submitting_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HackerParsed {
    pub hca_id: String,
    pub email: String,
    pub first_name: String,
    pub hackatime_token: String,
    pub user_id: i32,
    pub join_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub slack_id: String,
    pub friend_code: i32,
    pub reminders_opt_out: bool,
    pub dms_opt_out: bool,
    pub invite_token: String,
}

impl Into<HackerParsed> for DBHacker {
    fn into(self) -> HackerParsed {
        HackerParsed {
            hca_id: self.hca_id.unwrap_or_default(),
            email: self.email.unwrap_or_default(),
            first_name: self.first_name.unwrap_or_default(),
            hackatime_token: match self.hackatime_user_id {
                Some(_) => {
                    crypto::decrypt_or_empty(self.user_id.unwrap_or_default(), self.hackatime_token)
                }
                None => String::new(),
            },
            user_id: self.user_id.unwrap_or_default(),
            join_at: self.join_at,
            last_login_at: self.last_login_at,
            slack_id: self.slack_id.unwrap_or_default(),
            friend_code: self.friend_code.unwrap_or_default(),
            reminders_opt_out: self.reminders_opt_out.unwrap_or_default(),
            dms_opt_out: self.dms_opt_out.unwrap_or_default(),
            invite_token: self.invite_token.unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectParsed {
    pub project_id: i32,
    pub name: String,
    pub description: String,
    pub screenshot_url: String,
    pub code_url: String,
    pub playable_url: String,
    pub games: Vec<String>,
    pub user1: i32,
    pub user2: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub shipped_at: Option<NaiveDateTime>,
    pub approved_at: Option<NaiveDateTime>,
    pub review_status: Option<String>,
}

impl Into<ProjectParsed> for DBProject {
    fn into(self) -> ProjectParsed {
        ProjectParsed {
            project_id: self.project_id.unwrap_or_default(),
            name: self.name.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
            screenshot_url: self.screenshot_url.unwrap_or_default(),
            code_url: self.code_url.unwrap_or_default(),
            playable_url: self.playable_url.unwrap_or_default(),
            games: self.games.unwrap_or_default(),
            user1: self.user1.unwrap_or_default(),
            user2: self.user2,
            created_at: self.created_at,
            shipped_at: self.shipped_at,
            approved_at: self.approved_at,
            review_status: self.review_status,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserProjectParsed {
    pub user_id: i32,
    pub project_id: i32,
    pub first_time: bool,
    pub hours_needed: f32,
    pub hours_logged: f32,
    pub confirmed_at: Option<NaiveDateTime>,
    pub ship_ok_at: Option<NaiveDateTime>,
    pub hackatime_project_names: Vec<String>,
    pub airtable_record_id: Option<String>,
    pub submitting_at: Option<NaiveDateTime>,
}

impl Into<UserProjectParsed> for DBUserProject {
    fn into(self) -> UserProjectParsed {
        UserProjectParsed {
            user_id: self.user_id.unwrap_or_default(),
            project_id: self.project_id.unwrap_or_default(),
            first_time: self.first_time.unwrap_or_default(),
            hours_needed: self.hours_needed.unwrap_or_default(),
            hours_logged: self.hours_logged.unwrap_or_default(),
            confirmed_at: self.confirmed_at,
            ship_ok_at: self.ship_ok_at,
            hackatime_project_names: self.hackatime_project_names.unwrap_or_default(),
            airtable_record_id: self.airtable_record_id,
            submitting_at: self.submitting_at,
        }
    }
}

#[derive(serde::Serialize)]
pub struct HackatimeTrackedProjectInfo {
    pub name: String,
    pub hours: f32,
    pub unusable: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ClientMe {
    pub first_name: String,
    pub friend_code: i32,
    pub invite_token: String,
    pub hackatime_linked: bool,
    pub ysws_eligible: bool,
    pub verification_status: String,
    pub reminders: bool,
    pub dms: bool,
}

#[derive(serde::Serialize)]
pub struct ClientMemberInfo {
    pub user_id: i32,
    pub name: String,
    pub hours_logged: Option<f32>,
    pub hours_needed: f32,
    pub hours_target: f32,
    pub next_credits_at: f32,
    pub first_time: bool,
    pub confirmed: bool,
    pub ready_to_ship: bool,
    pub submitting: bool,
    pub hackatime: HackatimeStatus,
}

#[derive(serde::Serialize)]
pub struct ClientProjectInfo {
    pub project_id: i32,
    pub name: String,
    pub description: String,
    pub screenshot_url: String,
    pub code_url: String,
    pub playable_url: String,
    pub games: Vec<String>,
    pub shipped: bool,
    pub review_status: Option<String>,
    pub hackatime_project_names: Vec<String>,
    pub credit_budget: u32,
    pub credits_earned: u32,
    pub credits_spent: u32,
    pub credits_per_block: u32,
    pub me: ClientMemberInfo,
    pub friend: Option<ClientMemberInfo>,
    pub join_requests: Vec<ClientJoinRequest>,
}

#[derive(serde::Serialize)]
pub struct ClientJoinRequest {
    pub user_id: i32,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct ClientOutgoingRequest {
    pub project_id: i32,
    pub project_name: String,
    pub owner_name: String,
}

#[derive(serde::Serialize)]
pub struct ClientDashboard {
    pub hackatime: HackatimeStatus,
    pub projects: Vec<ClientProjectInfo>,
    pub join_requests: Vec<ClientOutgoingRequest>,
}

#[derive(serde::Serialize)]
pub struct ClientInvite {
    pub name: String,
    pub project_name: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ClientGame {
    pub name: &'static str,
    pub credits: u8,
    pub copies_label: String,
    pub image_url: String,
}

#[derive(serde::Serialize)]
pub struct ClientCatalog {
    pub credits_per_block: u32,
    pub games: Vec<ClientGame>,
}
