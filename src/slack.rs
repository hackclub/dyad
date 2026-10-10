use std::sync::LazyLock;

use log::warn;
use serde_json::{Value, json};

static TOKEN: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("SLACK_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
});

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

pub fn init() {
    if TOKEN.is_none() {
        warn!("SLACK_BOT_TOKEN not set; Slack DMs are disabled");
    }
}

pub fn dm(slack_id: &str, text: String) {
    let Some(token) = TOKEN.as_ref() else { return };
    if slack_id.is_empty() {
        return;
    }
    let slack_id = slack_id.to_string();
    tokio::spawn(async move {
        if let Err(e) = send(token, &slack_id, &text).await {
            warn!("Slack DM to {slack_id} failed: {e:?}");
        }
    });
}

async fn send(token: &str, channel: &str, text: &str) -> anyhow::Result<()> {
    let response: Value = CLIENT
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&json!({ "channel": channel, "text": text }))
        .send()
        .await?
        .json()
        .await?;
    if response["ok"].as_bool() != Some(true) {
        anyhow::bail!("{}", response["error"]);
    }
    Ok(())
}
