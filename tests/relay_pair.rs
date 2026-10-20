//! Two daemons on one machine exchange a message through a loopback `iroh-relay`.
//! On Linux, every socket those daemons open must stay on loopback.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FORBIDDEN: &[&str] = &["reasonmachines.com", "n0.iroh.link", "n0.computer", "dns.iroh.link", "iroh.link"];

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_oan")
}

fn cmd(home: &Path, machine: &str) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(bin());
    c.env("OAN_HOME", home)
        .env("OAN_MACHINE", machine)
        .env("OAN_NO_MDNS", "1")
        .env("OAN_NO_SERVICE", "1")
        .env("OAN_NO_UPDATE", "1")
        .env("OAN_NO_TRAY", "1")
        .env("OAN_NO_PROMPT", "1")
        .env("HOME", home.join("unix-home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    c
}

async fn run(home: &Path, machine: &str, args: &[&str]) -> (bool, String) {
    let mut c = cmd(home, machine);
    c.args(args);
    let out = tokio::time::timeout(Duration::from_secs(45), c.output())
        .await
        .unwrap_or_else(|_| panic!("timed out: oan {}", args.join(" ")))
        .unwrap_or_else(|e| panic!("spawn: {e}"));
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn ticket(text: &str) -> String {
    text.split_whitespace()
        .find(|w| w.contains("rn1-"))
        .unwrap_or_else(|| panic!("no team link in:\n{text}"))
        .trim_matches(|c: char| c == '"' || c == '\'')
        .to_string()
}

fn assert_clean(label: &str, text: &str) {
    let lower = text.to_ascii_lowercase();
    for host in FORBIDDEN {
        assert!(!lower.contains(host), "{label} names {host}:\n{text}");
    }
}

#[cfg(target_os = "linux")]
fn loopback(ip: &str) -> bool {
    ip == "127.0.0.1" || ip == "::1" || ip == "0.0.0.0" || ip == "::" || ip.starts_with("127.")
}

#[cfg(target_os = "linux")]
fn parse_addr(s: &str) -> Option<(String, u16)> {
    let (ip, port) = s.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    if ip.len() == 8 {
        let n = u32::from_str_radix(ip, 16).ok()?;
        let b = n.to_le_bytes();
        return Some((format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]), port));
    }
    if ip.len() == 32 {
        let mut bytes = [0u8; 16];
        for i in 0..4 {
            let word = u32::from_str_radix(&ip[i * 8..i * 8 + 8], 16).ok()?;
            bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        return Some((std::net::Ipv6Addr::from(bytes).to_string(), port));
    }
    None
}

#[cfg(target_os = "linux")]
fn socket_inodes(pid: u32) -> Vec<u64> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(format!("/proc/{pid}/fd")) else { return out };
    for ent in dir.flatten() {
        let Ok(link) = std::fs::read_link(ent.path()) else { continue };
        let Some(name) = link.to_str() else { continue };
        let Some(rest) = name.strip_prefix("socket:[") else { continue };
        let Some(inode) = rest.strip_suffix(']') else { continue };
        if let Ok(n) = inode.parse() {
            out.push(n);
        }
    }
    out
}

#[cfg(target_os = "linux")]
fn remotes(pid: u32) -> Vec<String> {
    let inodes: std::collections::HashSet<u64> = socket_inodes(pid).into_iter().collect();
    if inodes.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for file in ["/proc/net/tcp", "/proc/net/udp", "/proc/net/tcp6", "/proc/net/udp6"] {
        let Ok(text) = std::fs::read_to_string(file) else { continue };
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 10 {
                continue;
            }
            let Ok(inode) = cols[9].parse::<u64>() else { continue };
            if !inodes.contains(&inode) {
                continue;
            }
            let Some((ip, port)) = parse_addr(cols[2]) else { continue };
            if port == 0 || loopback(&ip) {
                continue;
            }
            found.push(format!("{ip}:{port}"));
        }
    }
    found
}

#[cfg(target_os = "linux")]
fn oan_pids() -> Vec<u32> {
    let want = PathBuf::from(bin());
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/proc") else { return out };
    for ent in dir.flatten() {
        let Ok(pid) = ent.file_name().to_str().unwrap_or("").parse::<u32>() else { continue };
        let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) else { continue };
        if exe == want {
            out.push(pid);
        }
    }
    out
}

/// Records any non-loopback socket the `oan` processes open while the flag is clear.
#[cfg(target_os = "linux")]
fn watch_sockets() -> (Arc<AtomicBool>, Arc<Mutex<Vec<String>>>) {
    let stop = Arc::new(AtomicBool::new(false));
    let bad = Arc::new(Mutex::new(Vec::new()));
    let (stop_t, bad_t) = (stop.clone(), bad.clone());
    std::thread::spawn(move || {
        while !stop_t.load(Ordering::Relaxed) {
            for pid in oan_pids() {
                for peer in remotes(pid) {
                    bad_t.lock().unwrap().push(format!("pid {pid} -> {peer}"));
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    (stop, bad)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_daemons_talk_through_a_local_relay() {
    let mut config = iroh_relay::server::ServerConfig::default();
    config.relay = Some(iroh_relay::server::RelayConfig::new((Ipv4Addr::LOCALHOST, 0)));
    config.quic = None;
    config.metrics_addr = None;
    let server = iroh_relay::server::Server::spawn(config).await.expect("local relay");
    let addr = server.http_addr().expect("relay bound an http port");
    let relay = format!("http://{addr}");
    assert!(relay.starts_with("http://127.0.0.1:"), "{relay}");

    let root = std::env::temp_dir().join(format!("oan-relay-pair-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let alice = root.join("alice");
    let bob = root.join("bob");
    for home in [&alice, &bob] {
        std::fs::create_dir_all(home.join("unix-home")).unwrap();
    }

    #[cfg(target_os = "linux")]
    let (stop_watch, bad_peers) = watch_sockets();

    let (ok, init_out) =
        run(&alice, "alice-box", &["init", "localnet", "--as", "alice", "--relay", &relay, "--no-setup"]).await;
    assert!(ok, "init failed:\n{init_out}\n{}", read_log(&alice));
    assert_clean("init", &init_out);
    let link = ticket(&init_out);
    assert!(link.starts_with("rn1-"), "{link}");

    let (ok, join_out) = run(&bob, "bob-box", &["join", &link, "--as", "bob", "--no-setup"]).await;
    assert!(ok, "join failed:\n{join_out}\n{}", read_log(&bob));
    assert!(join_out.contains("Joined team localnet as bob"), "{join_out}");
    assert_clean("join", &join_out);

    let mut listen = cmd(&bob, "bob-box");
    listen.args(["listen", "--as", "terminal", "--wait", "30"]);
    let listen = listen.spawn().expect("listen");

    let mut saw_session = false;
    for _ in 0..40 {
        let (ok, who) = run(&alice, "alice-box", &["who", "--json"]).await;
        if ok && who.contains("terminal") && who.contains("bob") {
            saw_session = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(saw_session, "bob's terminal session never showed up\n{}", read_log(&alice));

    let (ok, sent) = run(&alice, "alice-box", &["send", "bob/terminal", "ping-from-alice"]).await;
    assert!(ok, "send failed:\n{sent}\n{}", read_log(&alice));
    assert!(sent.contains("bob/terminal"), "{sent}");
    assert_clean("send", &sent);

    let finished =
        tokio::time::timeout(Duration::from_secs(20), listen.wait_with_output()).await.expect("listen timed out");
    let finished = finished.expect("listen");
    let heard = format!("{}{}", String::from_utf8_lossy(&finished.stdout), String::from_utf8_lossy(&finished.stderr));
    assert!(heard.contains("ping-from-alice"), "bob did not hear the message:\n{heard}");
    assert_clean("listen", &heard);

    let _ = run(&alice, "alice-box", &["stop"]).await;
    let _ = run(&bob, "bob-box", &["stop"]).await;

    #[cfg(target_os = "linux")]
    {
        stop_watch.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(50));
        let peers = bad_peers.lock().unwrap().clone();
        assert!(peers.is_empty(), "a daemon connected outside loopback:\n{}", peers.join("\n"));
    }

    for home in [&alice, &bob] {
        assert_clean("daemon.log", &read_log(home));
    }
    drop(server);
    let _ = std::fs::remove_dir_all(&root);
}

fn read_log(home: &Path) -> String {
    std::fs::read_to_string(home.join("daemon.log")).unwrap_or_default()
}

#[test]
fn the_binary_names_this_product_and_not_the_upstream_one() {
    let bytes = std::fs::read(bin()).expect("oan binary");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("Open Agent Network"), "binary is missing the product name");
    assert!(text.contains("org.openagentnetwork.daemon"));
    // The refusal list has to name the host. A full upstream relay URL must not.
    assert!(text.contains("reasonmachines.com"));
    for needle in ["Reason Network", "Reason Machines", "compute.reasonmachines.com"] {
        assert!(!text.contains(needle), "oan contains {needle}");
    }
}
