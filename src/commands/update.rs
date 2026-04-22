//! `creo update` – check GitHub for newer releases and notify the user.
//!
//! For safety the binary is *not* overwritten in-place automatically;
//! we just print the latest version and the install command. (Self-
//! replacing binaries on Linux is fine but feels surprising; this can
//! be promoted to true self-update in a follow-up.)

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::utils::output;

const RELEASES_URL: &str = "https://api.github.com/repos/creo-cli/creo/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

pub async fn run() -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(RELEASES_URL)
        .header("User-Agent", "creo-cli")
        .send()
        .await
        .context("failed to query GitHub releases")?;
    if !resp.status().is_success() {
        output::warn(&format!("GitHub returned {}", resp.status()));
        return Ok(());
    }
    let r: Release = resp.json().await.context("unexpected release shape")?;
    let current = env!("CARGO_PKG_VERSION");
    let latest = r.tag_name.trim_start_matches('v');
    if latest == current {
        output::success(&format!("creo ist aktuell ({current})"));
    } else {
        output::info(&format!(
            "Neue Version verfügbar: {latest}  (du hast {current})"
        ));
        println!("  • cargo install creo");
        println!("  • npm i -g creo");
        println!("  • curl -fsSL https://creo.dev/install.sh | sh");
        println!();
        output::step(&format!("Release: {}", r.html_url));
    }
    Ok(())
}
