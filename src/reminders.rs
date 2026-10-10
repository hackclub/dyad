use std::collections::HashMap;

use hc_helper::hackatime::data::Project;
use log::warn;
use sqlx::{Pool, Postgres, types::chrono::Utc};

use crate::{
    db::requests,
    hackatime::{get_summaries, round_hours},
    slack,
};

const QUIET_DAYS: i64 = 5;

pub async fn poll(pool: Pool<Postgres>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
    loop {
        interval.tick().await;
        if let Err(e) = check(&pool).await {
            warn!("Inactivity check failed: {e:?}");
        }
    }
}

async fn check(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let mut summaries: HashMap<i32, Option<Vec<Project>>> = HashMap::new();
    for row in requests::get_activity_rows(pool).await? {
        if !summaries.contains_key(&row.user_id) {
            let fetched = get_summaries(&row.hackatime_token)
                .await
                .inspect_err(|e| {
                    warn!(
                        "Couldn't get Hackatime hours for user {}: {e:?}",
                        row.user_id
                    )
                })
                .ok();
            summaries.insert(row.user_id, fetched);
        }
        let Some(Some(projects)) = summaries.get(&row.user_id) else {
            continue;
        };
        let hours = round_hours(
            projects
                .iter()
                .filter(|p| row.hackatime_project_names.contains(&p.name))
                .map(|p| p.total_duration_seconds)
                .sum(),
        );

        match (row.last_seen_hours, row.last_active_at) {
            (Some(seen), Some(active)) if hours <= seen => {
                if row.nudged_at.is_none()
                    && (Utc::now().naive_utc() - active).num_days() >= QUIET_DAYS
                {
                    slack::dm(
                        &row.slack_id,
                        format!(
                            "It's been {QUIET_DAYS} days since you last logged time on \"{}\". Jump back in when you can! (You can turn these reminders off on your Dyad dashboard.)",
                            row.project_name
                        ),
                    );
                    requests::set_nudged(row.project_id, row.user_id, pool).await?;
                }
            }
            _ => requests::set_activity(row.project_id, row.user_id, hours, pool).await?,
        }
    }
    Ok(())
}
