use std::collections::{BTreeMap, HashMap};

use hc_helper::airtable::{AirtableTable, get_records};
use log::{info, warn};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Pool, Postgres};

use crate::{
    airtable,
    db::requests::{self, ReviewRow},
    slack::{self, plain},
};

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
    #[serde(rename = "Pair Complete", default)]
    pair_complete: bool,
}

pub async fn poll(table: AirtableTable, pool: Pool<Postgres>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
    loop {
        interval.tick().await;
        if !airtable::ready() {
            continue;
        }
        if let Err(e) = check(&table, &pool).await {
            warn!("Review status check failed: {}", airtable::describe(&e));
        }
    }
}

async fn check(table: &AirtableTable, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let rows = requests::get_review_rows(pool).await?;
    if rows.is_empty() {
        return Ok(());
    }

    let ids: Vec<String> = rows.iter().map(|r| r.record_id.clone()).collect();
    let records: HashMap<String, StatusFields> = get_records::<StatusFields>(table.clone(), &ids)
        .await?
        .into_iter()
        .collect();

    let missing = rows
        .iter()
        .filter(|r| !records.contains_key(&r.record_id))
        .count();
    let hold_send_backs = missing >= 3 && missing * 2 > rows.len();
    if hold_send_backs {
        warn!(
            "{missing} of {} Airtable records are missing, so holding off on sending anything back. Check the Airtable table.",
            rows.len()
        );
    }

    let mut by_project: BTreeMap<i32, Vec<&ReviewRow>> = BTreeMap::new();
    for row in &rows {
        by_project.entry(row.project_id).or_default().push(row);
    }

    for (project_id, rows) in by_project {
        let result = if rows.iter().any(|r| !records.contains_key(&r.record_id)) {
            if hold_send_backs {
                Ok(())
            } else {
                send_back(table, project_id, &rows, &records, pool).await
            }
        } else if rows.iter().all(|r| r.shipped) {
            reconcile(table, project_id, &rows, &records, pool).await
        } else {
            untick(table, &rows, &records).await
        };
        if let Err(e) = result {
            warn!(
                "Couldn't check the review for project {project_id}: {}",
                airtable::describe(&e)
            );
        }
    }
    Ok(())
}

async fn untick(
    table: &AirtableTable,
    rows: &[&ReviewRow],
    records: &HashMap<String, StatusFields>,
) -> anyhow::Result<()> {
    let marked: Vec<String> = rows
        .iter()
        .filter(|r| records[&r.record_id].pair_complete)
        .map(|r| r.record_id.clone())
        .collect();
    if !marked.is_empty() {
        airtable::set_pair_complete(table, &marked, false).await?;
    }
    Ok(())
}

fn review_of(row: &ReviewRow, fields: &StatusFields) -> Option<Review> {
    let review = fields.status.as_str().and_then(Review::parse);
    let blank = fields.status.is_null() || fields.status.as_str() == Some("");
    if review.is_none() && !blank {
        warn!(
            "Airtable record {} for project {} has an unknown status {}",
            row.record_id, row.project_id, fields.status
        );
    }
    review
}

async fn reconcile(
    table: &AirtableTable,
    project_id: i32,
    rows: &[&ReviewRow],
    records: &HashMap<String, StatusFields>,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let unmarked: Vec<String> = rows
        .iter()
        .filter(|r| !records[&r.record_id].pair_complete)
        .map(|r| r.record_id.clone())
        .collect();
    if !unmarked.is_empty()
        && let Err(e) = airtable::set_pair_complete(table, &unmarked, true).await
    {
        warn!(
            "Couldn't mark project {project_id} as a complete pair: {}",
            airtable::describe(&e)
        );
    }

    let Some(combined) = rows
        .iter()
        .map(|r| review_of(r, &records[&r.record_id]))
        .collect::<Option<Vec<_>>>()
        .and_then(|r| r.into_iter().min())
    else {
        return Ok(());
    };
    update(project_id, combined, pool).await
}

async fn send_back(
    table: &AirtableTable,
    project_id: i32,
    rows: &[&ReviewRow],
    records: &HashMap<String, StatusFields>,
    pool: &Pool<Postgres>,
) -> anyhow::Result<()> {
    let missing: Vec<(i32, String)> = rows
        .iter()
        .filter(|r| !records.contains_key(&r.record_id))
        .map(|r| (r.user_id, r.record_id.clone()))
        .collect();
    for (_, record_id) in &missing {
        info!(
            "Airtable record {record_id} for project {project_id} was deleted, so sending it back"
        );
    }
    let Some(result) = requests::send_back(project_id, &missing, pool).await? else {
        return Ok(());
    };

    for record_id in &result.withdrawn_records {
        let Err(e) = airtable::delete_record(table, record_id).await else {
            continue;
        };
        warn!(
            "Couldn't withdraw Airtable record {record_id} for project {project_id}, so delete it by hand: {}",
            airtable::describe(&e)
        );
        if let Err(e) =
            airtable::set_pair_complete(table, std::slice::from_ref(record_id), false).await
        {
            warn!(
                "Couldn't unmark Airtable record {record_id} as a complete pair either: {}",
                airtable::describe(&e)
            );
        }
    }

    let project = match requests::get_project(project_id, pool).await {
        Ok(project) => format!("\"{}\"", plain(&project.name)),
        Err(e) => {
            warn!("Couldn't load project {project_id} for send-back DMs: {e:?}");
            "your project".into()
        }
    };
    let members = requests::get_project_members(project_id, pool).await?;
    let names = members
        .iter()
        .filter(|h| result.sent_back.contains(&h.user_id))
        .map(|h| plain(&h.first_name))
        .collect::<Vec<_>>()
        .join(" and ");
    let no_longer_shipped = if result.shipped {
        ", so it's no longer shipped"
    } else {
        ""
    };
    for hacker in &members {
        let message = if result.sent_back.contains(&hacker.user_id) {
            format!(
                "A reviewer sent back your submission for {project} on Dyad. Make your fixes, then press Ready to ship again."
            )
        } else if result.unreadied.contains(&hacker.user_id) {
            format!(
                "A reviewer sent back {names}'s submission for {project} on Dyad{no_longer_shipped}. Your Ready to ship was cleared so either of you can make changes. Press it again once you're both set."
            )
        } else {
            format!(
                "A reviewer sent back {names}'s submission for {project} on Dyad. They'll need to press Ready to ship again."
            )
        };
        if !hacker.dms_opt_out {
            slack::dm(&hacker.slack_id, message);
        }
    }
    Ok(())
}

async fn update(project_id: i32, combined: Review, pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let project = requests::get_project(project_id, pool).await?;
    if project.review_status.as_deref() == Some(combined.label()) {
        return Ok(());
    }
    requests::set_review_status(
        project_id,
        combined.label(),
        combined == Review::Approved,
        pool,
    )
    .await?;

    let project_name = plain(&project.name);
    let message = match combined {
        Review::Approved => format!("\"{project_name}\" was approved on Dyad! You will get a DM (might be a little while) with how to redeem your game(s)!"),
        Review::Error => format!(
            "There was a problem processing the submission for \"{project_name}\". A reviewer will take a look."
        ),
        _ => return Ok(()),
    };
    for hacker in requests::get_project_members(project_id, pool).await? {
        if !hacker.dms_opt_out {
            slack::dm(&hacker.slack_id, message.clone());
        }
    }
    Ok(())
}
