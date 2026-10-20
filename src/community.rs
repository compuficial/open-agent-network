//! Opt-in community relays. The default remains a relay the team runs itself.
//! Modified by compuficial, 2026.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::time::{Duration, Instant};

/// How many healthy relays a team keeps. iroh fails over inside that set.
pub const KEEP: usize = 3;
/// A team link stores the relay field in one length byte, so the joined URLs must fit.
const MAX_STORED: usize = 250;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    relays: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    url: String,
    operator: String,
    github: String,
    region: String,
    added: String,
    #[serde(default)]
    notes: Option<String>,
}

/// One relay from the published list, after its fields have been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunityRelay {
    pub url: String,
    pub operator: String,
    pub github: String,
    pub region: String,
    pub added: String,
    pub notes: Option<String>,
}

/// A relay that answered `/healthz`, with how long that took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    pub relay: CommunityRelay,
    pub rtt: Duration,
}

/// Where `relays.json` is fetched from. `OAN_COMMUNITY_RELAYS_URL` overrides the raw file on `main`.
pub fn list_url() -> String {
    match std::env::var(format!("{}_COMMUNITY_RELAYS_URL", crate::fork::ENV_PREFIX)) {
        Ok(url) if !url.trim().is_empty() => url,
        _ => format!("https://raw.githubusercontent.com/{}/main/relays.json", crate::fork::REPO),
    }
}

pub fn parse_document(text: &str) -> Result<Vec<CommunityRelay>> {
    let file: Document = serde_json::from_str(text).context("community relay list is not valid JSON")?;
    file.relays.into_iter().enumerate().map(|(i, entry)| entry.checked(i)).collect()
}

impl Entry {
    fn checked(self, index: usize) -> Result<CommunityRelay> {
        let url = self.url.trim().to_string();
        let operator = self.operator.trim().to_string();
        let github = self.github.trim().trim_start_matches('@').to_string();
        let region = self.region.trim().to_string();
        let added = self.added.trim().to_string();
        let notes = self.notes.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        let at = index + 1;
        if !url.starts_with("https://") || url.contains(char::is_whitespace) {
            bail!("community relay {at} must be an https URL");
        }
        if crate::fork::forbidden_relay(&url) {
            bail!("community relay {at} is not allowed ({url})");
        }
        if operator.is_empty() || github.is_empty() || region.is_empty() {
            bail!("community relay {at} needs an operator, a GitHub handle, and a region");
        }
        if !date_ok(&added) {
            bail!("community relay {at} needs an added date as YYYY-MM-DD");
        }
        Ok(CommunityRelay { url, operator, github, region, added, notes })
    }
}

fn ensure_listed(entries: &[CommunityRelay]) -> Result<()> {
    if entries.is_empty() {
        bail!("The community relay list is empty. Host one (see docs/RELAY.md) or pass --relay with a relay you run.");
    }
    Ok(())
}

fn date_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Fastest healthy relays, at most [`KEEP`]. `probes[i]` is how long `/healthz` took, or `None` if it failed.
pub fn rank(entries: Vec<CommunityRelay>, probes: &[Option<Duration>]) -> Vec<Ranked> {
    let mut ranked: Vec<Ranked> = entries
        .into_iter()
        .zip(probes.iter().copied())
        .filter_map(|(relay, rtt)| rtt.map(|rtt| Ranked { relay, rtt }))
        .collect();
    ranked.sort_by_key(|r| r.rtt);
    ranked.truncate(KEEP);
    ranked
}

/// The string stored on the team and in the invite. Drops the longest URLs until it fits.
pub fn stored(ranked: &[Ranked]) -> Result<String> {
    let mut chosen: Vec<&Ranked> = ranked.iter().collect();
    while !chosen.is_empty() {
        let text = chosen.iter().map(|r| r.relay.url.as_str()).collect::<Vec<_>>().join(" ");
        if text.len() <= MAX_STORED {
            return Ok(text);
        }
        let longest = chosen.iter().enumerate().max_by_key(|(_, r)| r.relay.url.len()).map(|(i, _)| i).unwrap_or(0);
        chosen.remove(longest);
    }
    bail!("community relay URLs do not fit in a team link")
}

/// Fetches the list, keeps the fastest few that answer `/healthz`, and prints who runs each one.
pub async fn choose(json: bool) -> Result<String> {
    let url = list_url();
    let text = fetch_text(&url).await.with_context(|| format!("fetching community relays from {url}"))?;
    let entries = parse_document(&text)?;
    ensure_listed(&entries)?;
    let probes = health(&entries).await;
    let ranked = rank(entries, &probes);
    if ranked.is_empty() {
        bail!("No community relay answered GET /healthz.");
    }
    let stored = stored(&ranked)?;
    let kept: Vec<&Ranked> = ranked.iter().filter(|r| stored.split_whitespace().any(|u| u == r.relay.url)).collect();
    report(&kept, json);
    Ok(stored)
}

fn report(chosen: &[&Ranked], json: bool) {
    use std::fmt::Write;
    let mut out = String::from("Community relays (fastest first). Each is run by the operator named here:\n");
    for r in chosen {
        let ms = r.rtt.as_millis();
        let _ =
            writeln!(out, "  {}  {} (@{})  {}  {ms}ms", r.relay.url, r.relay.operator, r.relay.github, r.relay.region);
    }
    if json { eprint!("{out}") } else { print!("{out}") }
}

async fn fetch_text(url: &str) -> Result<String> {
    let response = client()?.get(url).send().await.with_context(|| format!("GET {url}"))?;
    if !response.status().is_success() {
        bail!("GET {url} returned {}", response.status());
    }
    response.text().await.context("reading the community relay list")
}

async fn health(entries: &[CommunityRelay]) -> Vec<Option<Duration>> {
    let Ok(http) = client() else {
        return vec![None; entries.len()];
    };
    let mut out = Vec::with_capacity(entries.len());
    let mut tasks = tokio::task::JoinSet::new();
    for (i, entry) in entries.iter().enumerate() {
        let http = http.clone();
        let url = healthz(&entry.url);
        tasks.spawn(async move {
            let started = Instant::now();
            let ok = match http.get(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            };
            (i, ok.then_some(started.elapsed()))
        });
    }
    out.resize(entries.len(), None);
    while let Some(joined) = tasks.join_next().await {
        if let Ok((i, probe)) = joined {
            out[i] = probe;
        }
    }
    out
}

fn healthz(url: &str) -> String {
    format!("{}/healthz", url.trim_end_matches('/'))
}

fn client() -> Result<reqwest::Client> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .user_agent("open-agent-network")
        .build()
        .context("http client")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(url: &str) -> CommunityRelay {
        CommunityRelay {
            url: url.into(),
            operator: "Ada".into(),
            github: "ada".into(),
            region: "us-east".into(),
            added: "2026-10-20".into(),
            notes: None,
        }
    }

    #[test]
    fn the_published_list_starts_empty_and_matches_the_schema() {
        let list = parse_document(include_str!("../relays.json")).unwrap();
        assert!(list.is_empty());
        let err = ensure_listed(&list).unwrap_err().to_string();
        assert!(err.contains("empty"), "{err}");
        assert!(err.contains("--relay"), "{err}");
        let schema = include_str!("../relays.schema.json");
        for field in ["url", "operator", "github", "region", "added", "notes"] {
            assert!(schema.contains(&format!("\"{field}\"")), "{field}");
        }
        assert!(schema.contains("https://"));
    }

    #[test]
    fn a_community_entry_must_be_https_and_allowed() {
        let ok = r#"{"relays":[{"url":"https://relay.example","operator":"Ada","github":"@ada","region":"eu","added":"2026-10-20","notes":"lab"}]}"#;
        let parsed = parse_document(ok).unwrap();
        assert_eq!(parsed[0].github, "ada");
        assert_eq!(parsed[0].notes.as_deref(), Some("lab"));
        assert!(parse_document(r#"{"relays":[]}"#).unwrap().is_empty());
        for bad in [
            r#"{"relays":[{"url":"http://relay.example","operator":"Ada","github":"ada","region":"eu","added":"2026-10-20"}]}"#,
            r#"{"relays":[{"url":"https://use1-1.relay.n0.iroh.link","operator":"Ada","github":"ada","region":"eu","added":"2026-10-20"}]}"#,
            r#"{"relays":[{"url":"https://relay.example.n0.computer","operator":"Ada","github":"ada","region":"eu","added":"2026-10-20"}]}"#,
            r#"{"relays":[{"url":"https://relay.example","operator":"","github":"ada","region":"eu","added":"2026-10-20"}]}"#,
            r#"{"relays":[{"url":"https://relay.example","operator":"Ada","github":"ada","region":"eu","added":"20-10-2026"}]}"#,
            r#"{"relays":[{"url":"https://relay.example","operator":"Ada","github":"ada","region":"eu","added":"2026-10-20","extra":1}]}"#,
        ] {
            assert!(parse_document(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_fastest_three_are_kept_and_must_fit_in_a_ticket() {
        let entries = vec![
            entry("https://slow.example"),
            entry("https://fast.example"),
            entry("https://mid.example"),
            entry("https://also.example"),
            entry("https://down.example"),
        ];
        let probes = [
            Some(Duration::from_millis(80)),
            Some(Duration::from_millis(10)),
            Some(Duration::from_millis(30)),
            Some(Duration::from_millis(40)),
            None,
        ];
        let ranked = rank(entries, &probes);
        assert_eq!(
            ranked.iter().map(|r| r.relay.url.as_str()).collect::<Vec<_>>(),
            ["https://fast.example", "https://mid.example", "https://also.example"]
        );
        assert_eq!(stored(&ranked).unwrap(), "https://fast.example https://mid.example https://also.example");

        let huge = "https://".to_string() + &"a".repeat(300) + ".example";
        let ranked = vec![
            Ranked { relay: entry(&huge), rtt: Duration::from_millis(1) },
            Ranked { relay: entry("https://short.example"), rtt: Duration::from_millis(2) },
        ];
        assert_eq!(stored(&ranked).unwrap(), "https://short.example");
    }

    #[test]
    fn the_default_list_is_this_repo_on_main() {
        assert_eq!(
            format!("https://raw.githubusercontent.com/{}/main/relays.json", crate::fork::REPO),
            "https://raw.githubusercontent.com/compuficial/open-agent-network/main/relays.json"
        );
    }
}
