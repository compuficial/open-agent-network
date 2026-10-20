//! Open Agent Network policy. Wire identifiers (ALPN `tincan/1`, ticket prefix `rn1-`) stay in `net` and `team`.
//! Modified by compuficial, 2026.
//!
//! This build never selects a public relay. The only hosts named here are ones a configured relay must not be.

use anyhow::{Result, bail};

pub const PRODUCT: &str = "Open Agent Network";
pub const AGENT_NAME: &str = "oan";
pub const REPO: &str = "compuficial/open-agent-network";
/// State directory under the user's home (`~/.oan`).
pub const HOME_DIR: &str = ".oan";
pub const ENV_PREFIX: &str = "OAN";
pub const SERVICE_LABEL: &str = "org.openagentnetwork.daemon";
/// Directory name for a shortened Unix socket (`/tmp/oan-<uid>`).
pub const SHORT_NAME: &str = "oan";
pub const TAG_MESSAGE: &str = "oan-message";
pub const TAG_APPROVAL: &str = "oan-approval";
/// Frame tags spell this word. `render` defangs look-alikes so a message cannot forge a frame.
pub const TAG_WORD: &str = "oan";

/// Hosts a relay URL must not name. Compared case-insensitively as substrings.
const REFUSED_HOSTS: &[&str] = &["reasonmachines.com", "n0.iroh.link", "n0.computer", ".iroh.link"];

pub fn invite_line(link: &str) -> String {
    format!("Join {PRODUCT}: {link}")
}

/// The invite text is the ticket itself. An upstream join URL is still accepted when parsing: only the
/// fragment is read, and nothing is fetched.
pub fn join_url(ticket: &str) -> String {
    ticket.to_string()
}

pub fn forbidden_relay(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    REFUSED_HOSTS.iter().any(|h| u.contains(h))
}

/// `relay` must be an explicit self-hosted URL. Missing, `default`, and refused hosts are errors.
pub fn require_relay(relay: Option<&str>) -> Result<&str> {
    let Some(url) = relay.map(str::trim).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("default")) else {
        bail!(
            "{PRODUCT} needs a self-hosted relay. Run `{AGENT_NAME} init <team> --relay <url>` \
             (or set {ENV_PREFIX}_RELAY). There is no default relay."
        );
    };
    if forbidden_relay(url) {
        bail!("that relay is not allowed; use your own self-hosted relay");
    }
    let local = url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost");
    if !url.starts_with("https://") && !local {
        bail!("relay url must start with https:// (plain http only for 127.0.0.1 or localhost)");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_relay_is_an_error() {
        let err = require_relay(None).unwrap_err().to_string();
        assert!(err.contains("self-hosted relay"), "{err}");
        assert!(require_relay(Some("")).is_err());
        assert!(require_relay(Some("default")).is_err());
    }

    #[test]
    fn public_relays_are_refused() {
        for url in [
            "https://relay-sfo1.compute.reasonmachines.com",
            "https://use1-1.relay.n0.iroh.link/",
            "https://example.n0.computer/relay",
            "https://dns.iroh.link",
        ] {
            assert!(require_relay(Some(url)).is_err(), "{url}");
        }
        assert!(require_relay(Some("https://relay.example.com")).is_ok());
        assert!(require_relay(Some("http://127.0.0.1:3340")).is_ok());
        assert!(require_relay(Some("http://relay.example.com")).is_err());
    }
}
