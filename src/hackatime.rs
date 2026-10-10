use hc_helper::hackatime::data::{Project, get_hackatime_project_summaries_with_token};

pub fn round_hours(seconds: f64) -> f32 {
    ((seconds as f32 / 3600.0) * 100.0).round() / 100.0
}

pub async fn get_summaries(hackatime_token: &str) -> anyhow::Result<Vec<Project>> {
    if hackatime_token.is_empty() {
        return Ok(vec![]);
    }
    get_hackatime_project_summaries_with_token(hackatime_token).await
}

pub async fn get_hours_logged(
    hackatime_token: &str,
    hackatime_project_names: &[String],
) -> anyhow::Result<f32> {
    let total_duration_seconds: f64 = get_summaries(hackatime_token)
        .await?
        .iter()
        .filter(|p| hackatime_project_names.contains(&p.name))
        .map(|p| p.total_duration_seconds)
        .sum();

    Ok(round_hours(total_duration_seconds))
}
