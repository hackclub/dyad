use sqlx::types::chrono::NaiveDateTime;

#[derive(sqlx::FromRow)]
pub struct DBHacker {
    pub hca_id: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub hca_token: Option<String>,
    pub hackatime_token: Option<String>,
    pub user_id: Option<i32>,
    pub first_time: Option<bool>,
    pub join_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub slack_id: Option<String>,
    pub friend_code: Option<i32>
}

#[derive(sqlx::FromRow)]
pub struct DBProject {
    pub project_id: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub screenshot_url: Option<String>,
    pub user1: Option<i32>,
    pub user2: Option<i32>,
    pub shipped_at: Option<NaiveDateTime>,
    pub approved_at: Option<NaiveDateTime>,
}

#[derive(sqlx::FromRow)]
pub struct DBUserProject {
    pub user_id: Option<i32>,
    pub project_id: Option<i32>,
    pub hours_needed: Option<f32>,
    pub hours_logged: Option<f32>,
    pub confirmed_at: Option<NaiveDateTime>,
    pub ship_ok_at: Option<NaiveDateTime>,
    pub hackatime_project_names: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HackerParsed {
    pub hca_id: String,
    pub email: String,
    pub first_name: String,
    pub hca_token: String,
    pub hackatime_token: String,
    pub user_id: i32,
    pub first_time: bool,
    pub join_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub slack_id: String,
    pub friend_code: i32 
}

impl Into<HackerParsed> for DBHacker {
    fn into(self) -> HackerParsed {
        HackerParsed {
            hca_id: self.hca_id.unwrap_or_default(),
            email: self.email.unwrap_or_default(),
            first_name: self.first_name.unwrap_or_default(),
            hca_token: self.hca_token.unwrap_or_default(),
            hackatime_token: self.hackatime_token.unwrap_or_default(),
            user_id: self.user_id.unwrap_or_default(),
            first_time: self.first_time.unwrap_or_default(),
            join_at: self.join_at,
            last_login_at: self.last_login_at,
            slack_id: self.slack_id.unwrap_or_default(),
            friend_code: self.friend_code.unwrap_or_default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectParsed {
    pub project_id: i32,
    pub name: String,
    pub description: String,
    pub screenshot_url: String,
    pub user1: i32,
    pub user2: i32,
    pub shipped_at: Option<NaiveDateTime>,
    pub approved_at: Option<NaiveDateTime>,
}

impl Into<ProjectParsed> for DBProject {
    fn into(self) -> ProjectParsed {
        ProjectParsed {
            project_id: self.project_id.unwrap_or_default(),
            name: self.name.unwrap_or_default(),
            description: self.description.unwrap_or_default(),
            screenshot_url: self.screenshot_url.unwrap_or_default(),
            user1: self.user1.unwrap_or_default(),
            user2: self.user2.unwrap_or_default(),
            shipped_at: self.shipped_at,
            approved_at: self.approved_at,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserProjectParsed {
    pub user_id: i32,
    pub project_id: i32,
    pub hours_needed: f32,
    pub hours_logged: f32,
    pub confirmed_at: Option<NaiveDateTime>,
    pub ship_ok_at: Option<NaiveDateTime>,
    pub hackatime_project_names: Option<Vec<String>>,
}

impl Into<UserProjectParsed> for DBUserProject {
    fn into(self) -> UserProjectParsed {
        UserProjectParsed {
            user_id: self.user_id.unwrap_or_default(),
            project_id: self.project_id.unwrap_or_default(),
            hours_needed: self.hours_needed.unwrap_or_default(),
            hours_logged: self.hours_logged.unwrap_or_default(),
            confirmed_at: self.confirmed_at,
            ship_ok_at: self.ship_ok_at,
            hackatime_project_names: self.hackatime_project_names,
        }
    }
}
