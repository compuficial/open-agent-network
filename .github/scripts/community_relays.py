#!/usr/bin/env python3
"""Health checks for the community relay list.

`check-pr` runs on pull requests: every newly added URL must be https and answer GET /healthz.
`nightly` probes every entry. A relay that fails three checks in a row is removed from relays.json.
The caller opens a pull request when this process exits 2.
"""

import argparse
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

FAILURES_BEFORE_REMOVAL = 3
REFUSED_HOSTS = ("reasonmachines.com", "n0.iroh.link", "n0.computer", ".iroh.link")
REQUIRED = ("url", "operator", "github", "region", "added")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def load(path):
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(data, dict) or not isinstance(data.get("relays"), list):
        raise SystemExit(f"{path} must be an object with a relays array")
    return data


def healthz(url):
    return url.rstrip("/") + "/healthz"


def probe(url, timeout=5):
    request = urllib.request.Request(healthz(url), method="GET", headers={"User-Agent": "open-agent-network"})
    opener = urllib.request.build_opener(NoRedirect)
    try:
        with opener.open(request, timeout=timeout) as response:
            return response.status == 200
    except (urllib.error.URLError, TimeoutError, OSError):
        return False


def problems(entry):
    if not isinstance(entry, dict):
        return ["entry is not an object"]
    found = []
    url = str(entry.get("url", "")).strip()
    if not url.startswith("https://"):
        found.append(f"{url or '(missing url)'} must start with https://")
    lowered = url.lower()
    for host in REFUSED_HOSTS:
        if host in lowered:
            found.append(f"{url} uses a refused host")
            break
    for key in REQUIRED:
        if key == "url":
            continue
        if not str(entry.get(key, "")).strip():
            found.append(f"{url or 'entry'} is missing {key}")
    added = str(entry.get("added", "")).strip()
    if added and (len(added) != 10 or added[4] != "-" or added[7] != "-"):
        found.append(f"{url} has added {added}, expected YYYY-MM-DD")
    return found


def new_entries(base, head):
    known = {str(item.get("url", "")).strip() for item in base.get("relays", []) if isinstance(item, dict)}
    return [item for item in head["relays"] if str(item.get("url", "")).strip() not in known]


def advance(entries, failures, up):
    """Return (kept entries, updated failure counts, removed entries)."""
    kept = []
    nxt = {}
    removed = []
    for entry in entries:
        url = str(entry["url"]).strip()
        if up.get(url):
            kept.append(entry)
            continue
        count = int(failures.get(url, 0)) + 1
        if count >= FAILURES_BEFORE_REMOVAL:
            removed.append(entry)
        else:
            nxt[url] = count
            kept.append(entry)
    return kept, nxt, removed


def check_pr(base, head, probe_fn):
    errors = []
    for entry in new_entries(base, head):
        errors.extend(problems(entry))
        url = str(entry.get("url", "")).strip()
        if url.startswith("https://") and not any(host in url.lower() for host in REFUSED_HOSTS):
            if not probe_fn(url):
                errors.append(f"{url} did not return 200 from {healthz(url)}")
    return errors


def git_show(ref):
    import subprocess

    try:
        out = subprocess.check_output(["git", "show", f"{ref}:relays.json"], text=True, stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        return {"relays": []}
    return json.loads(out)


def cmd_check_pr(args):
    base = git_show(args.base)
    head = load(args.head)
    errors = check_pr(base, head, probe)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"community relay check ok ({len(new_entries(base, head))} new)")
    return 0


def cmd_nightly(args):
    document = load(args.relays)
    entries = document["relays"]
    state_path = Path(args.state)
    failures = {}
    if state_path.is_file():
        failures = json.loads(state_path.read_text(encoding="utf-8")).get("failures", {})
    if not entries:
        state_path.parent.mkdir(parents=True, exist_ok=True)
        state_path.write_text(json.dumps({"failures": {}}, indent=2) + "\n", encoding="utf-8")
        print("community relay list is empty")
        return 0
    up = {str(entry["url"]).strip(): probe(str(entry["url"]).strip()) for entry in entries}
    kept, nxt, removed = advance(entries, failures, up)
    state_path.parent.mkdir(parents=True, exist_ok=True)
    state_path.write_text(json.dumps({"failures": nxt}, indent=2) + "\n", encoding="utf-8")
    if not removed:
        print(f"checked {len(entries)}; none reached {FAILURES_BEFORE_REMOVAL} consecutive failures")
        return 0
    document["relays"] = kept
    Path(args.relays).write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    for entry in removed:
        print(f"remove {entry['url']} after {FAILURES_BEFORE_REMOVAL} failed health checks")
    return 2


def main(argv):
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)
    pr = sub.add_parser("check-pr")
    pr.add_argument("--base", required=True)
    pr.add_argument("--head", default="relays.json")
    pr.set_defaults(func=cmd_check_pr)
    night = sub.add_parser("nightly")
    night.add_argument("--relays", default="relays.json")
    night.add_argument("--state", required=True)
    night.set_defaults(func=cmd_nightly)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
