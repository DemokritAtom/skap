//! Tiny GitLab API helper – create a project under the authenticated user.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CreateProject<'a> {
    name: &'a str,
    visibility: &'a str,
}

#[derive(Deserialize)]
struct ProjectResponse {
    ssh_url_to_repo: Option<String>,
    http_url_to_repo: Option<String>,
}

pub async fn create_project(
    base_url: &str,
    token: &str,
    name: &str,
    private: bool,
) -> Result<String> {
    let visibility = if private { "private" } else { "public" };
    let url = format!("{}/api/v4/projects", base_url.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .post(&url)
        .header("User-Agent", "skap-cli")
        .header("PRIVATE-TOKEN", token)
        .json(&CreateProject { name, visibility })
        .send()
        .await
        .context("failed to call GitLab API")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("GitLab API error ({status}): {text}");
    }
    let parsed: ProjectResponse =
        serde_json::from_str(&text).context("unexpected GitLab response shape")?;
    parsed
        .ssh_url_to_repo
        .or(parsed.http_url_to_repo)
        .ok_or_else(|| anyhow::anyhow!("GitLab response missing repo URL"))
}
