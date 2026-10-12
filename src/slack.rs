use std::{sync::LazyLock, time::Duration};

use log::warn;
use serde_json::{Value, json};

static TOKEN: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("SLACK_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
});

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Slack HTTP client should build")
});

const ATTEMPTS: u32 = 3;

pub fn init() {
    if TOKEN.is_none() {
        warn!("SLACK_BOT_TOKEN not set; Slack DMs are disabled");
    }
}

pub fn enabled() -> bool {
    TOKEN.is_some()
}

pub fn plain(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            let joiner = matches!(c, '.' | ':' | '/' | '@').then_some('\u{2060}');
            std::iter::once(c).chain(joiner)
        })
        .collect()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn dm(slack_id: &str, text: String) {
    if TOKEN.is_none() || slack_id.is_empty() {
        return;
    }
    let slack_id = slack_id.to_string();
    tokio::spawn(async move {
        if let Err(e) = dm_now(&slack_id, &text).await {
            warn!("Slack DM to {slack_id} failed: {e:?}");
        }
    });
}

pub async fn dm_now(slack_id: &str, text: &str) -> anyhow::Result<()> {
    let Some(token) = TOKEN.as_ref() else {
        anyhow::bail!("Slack is not configured");
    };
    if slack_id.is_empty() {
        anyhow::bail!("No Slack ID");
    }
    let mut delay = Duration::from_secs(2);
    let mut attempt = 1;
    loop {
        match post(token, slack_id, text).await {
            Err(e) if attempt < ATTEMPTS => {
                warn!("Slack DM to {slack_id} failed, retrying: {e:?}");
                tokio::time::sleep(delay).await;
                delay *= 2;
                attempt += 1;
            }
            result => return result,
        }
    }
}

async fn post(token: &str, slack_id: &str, text: &str) -> anyhow::Result<()> {
    let response: Value = CLIENT
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&json!({ "channel": slack_id, "text": escape(text) }))
        .send()
        .await?
        .json()
        .await?;
    if response["ok"].as_bool() != Some(true) {
        anyhow::bail!("{}", response["error"]);
    }
    Ok(())
}
