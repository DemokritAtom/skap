//! Tiny GitHub API helper – just enough to create a repository under
//! the authenticated user.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CreateRepo<'a> {
    name: &'a str,
    private: bool,
    auto_init: bool,
}

#[derive(Deserialize)]
struct RepoResponse {
    ssh_url: String,
    clone_url: String,
}

/// Create a repo under the authenticated user. Returns the SSH URL
/// (preferred) or the HTTPS clone URL as a fallback.
pub async fn create_repo(token: &str, name: &str, private: bool) -> Result<String> {
    let body = CreateRepo {
        name,
        private,
        auto_init: false,
    };
    let resp = reqwest::Client::new()
        .post("https://api.github.com/user/repos")
        .header("User-Agent", "skap-cli")
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context("failed to call GitHub API")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("GitHub API error ({status}): {text}");
    }
    let parsed: RepoResponse =
        serde_json::from_str(&text).context("unexpected GitHub response shape")?;
    if !parsed.ssh_url.is_empty() {
        Ok(parsed.ssh_url)
    } else {
        Ok(parsed.clone_url)
    }
}
