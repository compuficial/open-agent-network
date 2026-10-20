//! User-facing trees must not carry the upstream product name. Legal attribution stays in the
//! license files, NOTICE, and the License section of README.md. The refusal list in `src/fork.rs`
//! names hosts only to reject them.

use std::path::Path;

const NEEDLES: &[&str] =
    &["Reason Network", "Reason Machines", "reasonmachines.com", "reason-network", "reason-machines"];

fn allowed(path: &str, needle: &str) -> bool {
    if path == "NOTICE" || path == "LICENSE-MIT" || path == "LICENSE-APACHE" {
        return true;
    }
    if path == "src/fork.rs" || path == "tests/branding.rs" || path == "tests/relay_pair.rs" {
        return true;
    }
    if path == "FORKING.md"
        && (needle == "reason-network" || needle == "reason-machines" || needle == "reasonmachines.com")
    {
        return true;
    }
    if path == "docs/RELAY.md" && needle == "reasonmachines.com" {
        return true;
    }
    // Apache-2.0 attribution: the upstream git path, and only that path. The License-section
    // test below checks that README does not use it anywhere else.
    if path == "README.md" && (needle == "reason-network" || needle == "reason-machines") {
        return true;
    }
    false
}

fn readme_attribution(text: &str) -> Result<(), String> {
    // A Windows checkout may have rewritten the file with CRLF.
    let text = text.replace("\r\n", "\n");
    let Some((body, tail)) = text.rsplit_once("\n## License\n") else {
        return Err("README needs a License section for the Apache-2.0 attribution".into());
    };
    for needle in NEEDLES {
        if body.contains(needle) {
            return Err(format!("README above the License section contains {needle}"));
        }
    }
    if !tail.contains("https://github.com/reason-machines/reason-network") || !tail.contains("v0.12.0") {
        return Err(format!("README license section is missing the upstream attribution:\n{tail}"));
    }
    for needle in ["Reason Network", "Reason Machines", "reasonmachines.com"] {
        if tail.contains(needle) {
            return Err(format!("README license section contains {needle}"));
        }
    }
    Ok(())
}

#[test]
fn readme_names_the_upstream_repo_only_in_the_license_section() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    readme_attribution(&text).unwrap();
}

#[test]
fn readme_attribution_accepts_windows_newlines() {
    let sample = "Welcome to the Open Agent Network.\r\n\r\n## License\r\n\r\n\
        Fork of https://github.com/reason-machines/reason-network v0.12.0.\r\n";
    readme_attribution(sample).unwrap();
}

#[test]
fn product_names_stay_in_the_legal_files_and_the_refusal_list() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    walk(root, root, &mut hits);
    assert!(hits.is_empty(), "upstream branding remains:\n{}", hits.join("\n"));
}

fn walk(root: &Path, dir: &Path, hits: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for ent in rd.flatten() {
        let path = ent.path();
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, hits);
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(text) = String::from_utf8(bytes) else { continue };
        for needle in NEEDLES {
            if text.contains(needle) && !allowed(&rel, needle) {
                hits.push(format!("{rel}: {needle}"));
            }
        }
    }
}
