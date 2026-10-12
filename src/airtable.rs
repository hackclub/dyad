use std::{
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{anyhow, bail};
use hc_helper::airtable::{AirtableTable, create_field, create_table, tables};
use log::{info, warn};
use reqwest::{RequestBuilder, Response, StatusCode, Url, header::RETRY_AFTER};
use serde_json::{Value, json};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Airtable HTTP client should build")
});

fn field_definitions() -> Vec<Value> {
    let text = |name: &str| json!({ "name": name, "type": "singleLineText" });
    vec![
        json!({ "name": "Code URL", "type": "url" }),
        json!({ "name": "Playable URL", "type": "url" }),
        text("First Name"),
        text("Last Name"),
        json!({ "name": "Email", "type": "email" }),
        json!({ "name": "Screenshot", "type": "multipleAttachments" }),
        json!({ "name": "Description", "type": "multilineText" }),
        text("Address (Line 1)"),
        text("Address (Line 2)"),
        text("City"),
        text("State / Province"),
        text("ZIP / Postal Code"),
        text("Country"),
        json!({ "name": "Birthday", "type": "date", "options": { "dateFormat": { "name": "iso" } } }),
        json!({ "name": "Justification - Hackatime Project Name(s) + Date Range(s)", "type": "multilineText" }),
        json!({ "name": "Games", "type": "multilineText" }),
        json!({ "name": "Partner Email", "type": "email" }),
        json!({ "name": "First Time", "type": "checkbox", "options": { "icon": "check", "color": "greenBright" } }),
        json!({ "name": "Hours", "type": "number", "options": { "precision": 2 } }),
        json!({ "name": "Pair Complete", "type": "checkbox", "options": { "icon": "check", "color": "greenBright" } }),
    ]
}

static READY: AtomicBool = AtomicBool::new(false);

const RATE_LIMIT_ATTEMPTS: u32 = 3;

pub fn configured(table: &AirtableTable) -> bool {
    !table.token.is_empty() && !table.base_id.is_empty() && !table.table_id_or_name.is_empty()
}

pub fn ready() -> bool {
    READY.load(Ordering::Acquire)
}

pub fn describe(e: &anyhow::Error) -> String {
    let status = e
        .chain()
        .find_map(|cause| cause.downcast_ref::<reqwest::Error>())
        .and_then(|e| e.status());
    let Some(status) = status else {
        return format!("{e:#}");
    };
    let kind = e.chain().find_map(|cause| {
        let body: Value = serde_json::from_str(
            cause
                .to_string()
                .strip_prefix("Airtable responded: ")?,
        )
        .ok()?;
        let error = &body["error"];
        error["type"].as_str().or(error.as_str()).map(str::to_string)
    });
    match kind {
        Some(kind) => format!("Airtable returned {status} {kind}"),
        None => format!("Airtable returned {status}"),
    }
}

pub async fn init_until_ready(table: AirtableTable) {
    let mut delay = Duration::from_secs(5);
    loop {
        match init(&table).await {
            Ok(()) => {
                READY.store(true, Ordering::Release);
                return;
            }
            Err(e) => warn!(
                "Airtable init failed, so submitting is off until it works. Retrying in {}s: {}",
                delay.as_secs(),
                describe(&e)
            ),
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(5 * 60));
    }
}

async fn init(table: &AirtableTable) -> anyhow::Result<()> {
    let fields = field_definitions();
    let existing = tables(&table.token, &table.base_id)
        .await?
        .into_iter()
        .find(|t| t.id == table.table_id_or_name || t.name == table.table_id_or_name);

    let Some(existing) = existing else {
        return create_table(
            &table.token,
            &table.base_id,
            &table.table_id_or_name,
            &fields,
        )
        .await;
    };

    for field in &fields {
        if !existing.fields.iter().any(|f| f.name == field["name"]) {
            create_field(&table.token, &table.base_id, &existing.id, field).await?;
        }
    }
    info!("Airtable table {} is ready", existing.name);
    Ok(())
}

fn records_url(table: &AirtableTable, record_id: Option<&str>) -> anyhow::Result<Url> {
    let mut url = Url::parse("https://api.airtable.com")?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("Airtable URL cannot be a base"))?;
        segments
            .clear()
            .push("v0")
            .push(&table.base_id)
            .push(&table.table_id_or_name);
        if let Some(record_id) = record_id {
            segments.push(record_id);
        }
    }
    Ok(url)
}

async fn send(request: RequestBuilder) -> anyhow::Result<Response> {
    let mut attempt = 1;
    loop {
        let response = request
            .try_clone()
            .ok_or_else(|| anyhow!("Airtable request can't be retried"))?
            .send()
            .await?;
        if response.status() != StatusCode::TOO_MANY_REQUESTS {
            return Ok(response);
        }
        if attempt == RATE_LIMIT_ATTEMPTS {
            bail!("Airtable kept rate limiting");
        }
        let wait = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(30u64)
            .min(60);
        tokio::time::sleep(Duration::from_secs(wait)).await;
        attempt += 1;
    }
}

pub async fn record_exists(table: &AirtableTable, record_id: &str) -> anyhow::Result<bool> {
    let response = send(
        CLIENT
            .get(records_url(table, Some(record_id))?)
            .bearer_auth(&table.token),
    )
    .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(false);
    }
    response.error_for_status()?;
    Ok(true)
}

pub async fn delete_record(table: &AirtableTable, record_id: &str) -> anyhow::Result<()> {
    let response = send(
        CLIENT
            .delete(records_url(table, Some(record_id))?)
            .bearer_auth(&table.token),
    )
    .await?;
    if response.status() != StatusCode::NOT_FOUND {
        response.error_for_status()?;
    }
    info!("Deleted Airtable record {record_id}");
    Ok(())
}

pub async fn set_pair_complete(
    table: &AirtableTable,
    record_ids: &[String],
    complete: bool,
) -> anyhow::Result<()> {
    for chunk in record_ids.chunks(10) {
        let records: Vec<Value> = chunk
            .iter()
            .map(|id| json!({ "id": id, "fields": { "Pair Complete": complete } }))
            .collect();
        send(
            CLIENT
                .patch(records_url(table, None)?)
                .bearer_auth(&table.token)
                .json(&json!({ "records": records })),
        )
        .await?
        .error_for_status()?;
    }
    Ok(())
}
