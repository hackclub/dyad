use std::collections::HashSet;

use log::warn;
use sqlx::{
    Pool, Postgres,
    types::chrono::{DateTime, NaiveDateTime, Utc},
};

use crate::{
    db::requests::{self, ActivityRow},
    hackatime::{Summaries, get_summaries_for},
    slack::{self, plain},
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

fn parse_time(stamp: &str) -> Option<NaiveDateTime> {
    DateTime::parse_from_rfc3339(stamp)
        .map(|t| t.naive_utc())
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H:%M:%S%.f").ok())
        .or_else(|| NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S%.f").ok())
        .or_else(|| {
            DateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S%.f %z")
                .map(|t| t.naive_utc())
                .ok()
        })
        .or_else(|| {
            let seconds: f64 = stamp.parse().ok()?;
            DateTime::from_timestamp(seconds as i64, 0).map(|t| t.naive_utc())
        })
}

fn last_activity(row: &ActivityRow, summaries: Option<&Summaries>) -> Option<Option<NaiveDateTime>> {
    let Some(Ok(projects)) = summaries else {
        return None;
    };
    let linked: Vec<_> = projects
        .iter()
        .filter(|p| row.hackatime_project_names.contains(&p.name))
        .collect();
    if linked.is_empty() {
        return None;
    }
    let stamps: Vec<&str> = linked
        .iter()
        .filter_map(|p| {
            p.most_recent_heartbeat
                .as_deref()
                .or(p.last_heartbeat.as_deref())
        })
        .collect();
    let parsed: Vec<NaiveDateTime> = stamps.iter().filter_map(|s| parse_time(s)).collect();
    if parsed.len() < stamps.len() {
        warn!("Couldn't parse some Hackatime heartbeat times {stamps:?}");
        if parsed.is_empty() {
            return None;
        }
    }
    Some(parsed.into_iter().max())
}

async fn check(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    if !slack::enabled() {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    let rows = requests::get_activity_rows(pool).await?;
    let tokens: HashSet<String> = rows
        .iter()
        .filter(|r| !r.hackatime_project_names.is_empty())
        .map(|r| r.hackatime_token.clone())
        .collect();
    let summaries = get_summaries_for(tokens).await;

    for row in rows {
        let linked = !row.hackatime_project_names.is_empty();
        let last = if linked {
            match last_activity(&row, summaries.get(&row.hackatime_token)) {
                Some(last) => last,
                None => continue,
            }
        } else {
            None
        };
        let since = last.map_or(row.confirmed_at, |last| last.max(row.confirmed_at));

        let quiet_days = (now - since).num_days();
        if quiet_days < QUIET_DAYS || row.nudged_at.is_some_and(|n| n >= since) {
            continue;
        }
        let project_name = plain(&row.project_name);
        let message = match (linked, last) {
            (true, Some(_)) => format!(
                "It's been {quiet_days} days since you last logged time on \"{project_name}\". Jump back in when you can! (You can turn these reminders off on your Dyad dashboard.)"
            ),
            (true, None) => format!(
                "You haven't logged any time on \"{project_name}\" yet. Jump in when you can! (You can turn these reminders off on your Dyad dashboard.)"
            ),
            (false, _) => format!(
                "You haven't linked a Hackatime project to \"{project_name}\" on Dyad yet. Link one from your dashboard so your time counts! (You can turn these reminders off on your Dyad dashboard.)"
            ),
        };
        if let Err(e) = requests::set_nudged(row.project_id, row.user_id, pool).await {
            warn!(
                "Couldn't record a reminder for project {} to user {}: {e:?}",
                row.project_id, row.user_id
            );
            continue;
        }
        if let Err(e) = slack::dm_now(&row.slack_id, &message).await {
            warn!(
                "Couldn't send a reminder for project {} to user {}: {e:?}",
                row.project_id, row.user_id
            );
        }
    }
    Ok(())
}
