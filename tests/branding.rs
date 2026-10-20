//! User-facing trees must not carry the upstream product name. Legal attribution stays in the
//! license files and NOTICE. The refusal list in `src/fork.rs` names hosts only to reject them.

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
    false
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
