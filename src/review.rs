use std::collections::{BTreeMap, HashMap};

use hc_helper::airtable::{AirtableTable, get_records};
use log::warn;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Pool, Postgres};

use crate::{db::requests, slack};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Review {
    Error,
    Pending,
    Processing,
    Approved,
}

impl Review {
    fn parse(status: &str) -> Option<Review> {
        let number: String = status
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        match number.as_str() {
            "0" => Some(Review::Error),
            "1" => Some(Review::Pending),
            "1.5" => Some(Review::Processing),
            "2" => Some(Review::Approved),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Review::Error => "error",
            Review::Pending => "pending",
            Review::Processing => "processing",
            Review::Approved => "approved",
        }
    }
}

#[derive(Deserialize)]
struct StatusFields {
    #[serde(rename = "Automation - Status", default)]
    status: Value,
}

pub async fn poll(table: AirtableTable, pool: Pool<Postgres>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
    loop {
        interval.tick().await;
        if let Err(e) = check(&table, &pool).await {
            warn!("Review status check failed: {e:?}");
        }
    }
}

async fn check(table: &AirtableTable, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let rows = requests::get_review_record_ids(pool).await?;
    if rows.is_empty() {
        return Ok(());
    }

    let ids: Vec<String> = rows.iter().map(|(_, id)| id.clone()).collect();
    let statuses: HashMap<String, Review> = get_records::<StatusFields>(table.clone(), &ids)
        .await?
        .into_iter()
        .filter_map(|(id, f)| Some((id, Review::parse(f.status.as_str()?)?)))
        .collect();

    let mut by_project: BTreeMap<i32, Vec<Option<Review>>> = BTreeMap::new();
    for (project_id, record_id) in &rows {
        by_project
            .entry(*project_id)
            .or_default()
            .push(statuses.get(record_id).copied());
    }

    for (project_id, reviews) in by_project {
        let Some(combined) = reviews
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|r| r.into_iter().min())
        else {
            continue;
        };
        let project = requests::get_project(project_id, pool).await?;
        if project.review_status.as_deref() == Some(combined.label()) {
            continue;
        }
        requests::set_review_status(
            project_id,
            combined.label(),
            combined == Review::Approved,
            pool,
        )
        .await?;

        let message = match combined {
            Review::Approved => format!("\"{}\" was approved on Dyad! You will get a DM (might be a little while) with how to redeem your game(s)!", project.name),
            Review::Error => format!(
                "There was a problem processing the submission for \"{}\". A reviewer will take a look.",
                project.name
            ),
            _ => continue,
        };
        for hacker in requests::get_project_members(project_id, pool).await? {
            slack::dm(&hacker.slack_id, message.clone());
        }
    }
    Ok(())
}
