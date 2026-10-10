use hc_helper::airtable::{AirtableTable, create_field, create_table, tables};
use log::info;
use serde_json::{Value, json};

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
    ]
}

pub async fn init(table: &AirtableTable) -> anyhow::Result<()> {
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
