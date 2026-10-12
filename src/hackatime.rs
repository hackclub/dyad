use std::{collections::HashMap, sync::Arc};

use hc_helper::hackatime::data::{Project, get_hackatime_project_summaries_with_token};
use log::{info, warn};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::error::{ApiError, rejected_by};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HackatimeStatus {
    Ok,
    Unlinked,
    Expired,
    Unreachable,
}

impl HackatimeStatus {
    pub fn my_error(self) -> ApiError {
        match self {
            HackatimeStatus::Unlinked => {
                ApiError::BadRequest("Link your Hackatime account first".into())
            }
            HackatimeStatus::Expired => ApiError::BadRequest(
                "Your Hackatime connection stopped working. Relink it from your dashboard.".into(),
            ),
            HackatimeStatus::Ok | HackatimeStatus::Unreachable => {
                ApiError::BadGateway("Couldn't reach Hackatime. Try again in a bit.".into())
            }
        }
    }
}

pub type Summaries = Result<Vec<Project>, HackatimeStatus>;

pub fn status(summaries: &Summaries) -> HackatimeStatus {
    match summaries {
        Ok(_) => HackatimeStatus::Ok,
        Err(status) => *status,
    }
}

pub fn round_hours(seconds: f64) -> f32 {
    ((seconds as f32 / 3600.0) * 100.0).round() / 100.0
}

pub fn seconds_in(projects: &[Project], hackatime_project_names: &[String]) -> f64 {
    projects
        .iter()
        .filter(|p| hackatime_project_names.contains(&p.name))
        .map(|p| p.total_duration_seconds)
        .sum()
}

pub fn hours_in(projects: &[Project], hackatime_project_names: &[String]) -> f32 {
    round_hours(seconds_in(projects, hackatime_project_names))
}

pub fn unusable(project: &Project) -> Option<&'static str> {
    if project.name.contains('.') {
        Some("Hackatime can't look up projects with a dot in the name. Rename it in Hackatime first.")
    } else if project.archived {
        Some("It's archived in Hackatime. Unarchive it first.")
    } else {
        None
    }
}

pub async fn get_summaries(hackatime_token: &str) -> Summaries {
    if hackatime_token.is_empty() {
        return Err(HackatimeStatus::Unlinked);
    }
    get_hackatime_project_summaries_with_token(hackatime_token)
        .await
        .map_err(|e| {
            if rejected_by(&e).is_some() {
                info!("Hackatime rejected a token: {e}");
                HackatimeStatus::Expired
            } else {
                warn!("Couldn't get Hackatime hours: {e:?}");
                HackatimeStatus::Unreachable
            }
        })
}

pub async fn get_summaries_for(
    hackatime_tokens: impl IntoIterator<Item = String>,
) -> HashMap<String, Summaries> {
    let permits = Arc::new(Semaphore::new(8));
    let mut set = JoinSet::new();
    for token in hackatime_tokens {
        let permits = permits.clone();
        set.spawn(async move {
            let _permit = permits.acquire_owned().await;
            let summaries = get_summaries(&token).await;
            (token, summaries)
        });
    }
    set.join_all().await.into_iter().collect()
}
