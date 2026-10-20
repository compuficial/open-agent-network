//! The Railway relay image stays on the same iroh-relay release as the `iroh` crate.

use std::path::Path;

fn read(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|err| panic!("read {path}: {err}"))
}

fn package_version(lock: &str, name: &str) -> String {
    let needle = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line == needle {
            let version = lines.next().unwrap_or_else(|| panic!("{name} has no version line"));
            let version = version.trim().strip_prefix("version = \"").unwrap_or_else(|| panic!("{version}"));
            return version.strip_suffix('"').unwrap_or(version).to_string();
        }
    }
    panic!("{name} is not in Cargo.lock");
}

#[test]
fn the_image_pins_the_iroh_release_and_stays_http_only() {
    let lock = read("Cargo.lock").replace("\r\n", "\n");
    let version = package_version(&lock, "iroh");
    assert_eq!(version, package_version(&lock, "iroh-relay"), "dev iroh-relay drifted from iroh");

    let dockerfile = read("deploy/railway/Dockerfile").replace("\r\n", "\n");
    assert!(
        dockerfile.contains(&format!("ARG IROH_RELAY_VERSION={version}")),
        "Dockerfile is not pinned to iroh {version}:\n{dockerfile}"
    );
    assert!(dockerfile.contains("cargo install \"iroh-relay@${IROH_RELAY_VERSION}\" --locked --features server"));

    let entry = read("deploy/railway/entrypoint.sh").replace("\r\n", "\n");
    assert!(!entry.contains('\r'));
    for line in [
        "enable_relay = true",
        "enable_quic_addr_discovery = false",
        "enable_metrics = false",
        "http_bind_addr = \"0.0.0.0:${port}\"",
        "[limits.client.rx]",
        "bytes_per_second = ${rx}",
        "max_burst_bytes = ${burst}",
        "OAN_RELAY_RX_BYTES_PER_SEC:-1048576",
        "OAN_RELAY_RX_BURST_BYTES:-4194304",
    ] {
        assert!(entry.contains(line), "entrypoint missing {line}");
    }
    assert!(entry.contains("PORT"), "entrypoint does not read PORT");
    assert!(!entry.contains("enable_quic_addr_discovery = true"));
    assert!(!entry.contains("[tls]"));
}

#[test]
fn railway_config_builds_that_image_and_checks_healthz() {
    let json = read("deploy/railway/railway.json");
    let toml = read("deploy/railway/railway.toml");
    for (label, text) in [("railway.json", json.as_str()), ("railway.toml", toml.as_str())] {
        assert!(text.contains("DOCKERFILE"), "{label} does not select the Dockerfile builder");
        assert!(text.contains("Dockerfile"), "{label} does not name the Dockerfile");
        assert!(text.contains("/healthz"), "{label} does not healthcheck /healthz");
        assert!(text.contains("ON_FAILURE"), "{label}");
    }
    let json = json.trim();
    assert!(json.starts_with('{') && json.ends_with('}'), "railway.json");
}
