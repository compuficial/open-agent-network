#!/bin/sh
# Installs Open Agent Network for macOS or Linux. It installs this engine on its own.
# A team needs a self-hosted relay: `oan init <team> --relay https://relay.example.org`.
#
#   curl -fsSL <this url> | sh                      install, then set up
#   curl -fsSL <this url> | sh -s -- <team link>    install, then join that team
#   curl -fsSL <this url> | sh -s -- <team link> --as <name>     the same, without questions (for agents)
#   curl -fsSL <this url> | sh -s -- init <team> --as <name>     install and start a team, without questions
#
# Environment: OAN_BIN_DIR (default ~/.local/bin), OAN_VERSION (a tag like v0.10.4; default latest), OAN_REPO,
# OAN_DOWNLOAD_BASE (a mirror serving the release files), OAN_NO_PROMPT (never ask anything: updates).
set -eu

repo="${OAN_REPO:-compuficial/open-agent-network}"
bin_dir="${OAN_BIN_DIR:-$HOME/.local/bin}"
version="${OAN_VERSION:-latest}"

say() { printf '%s\n' "$*"; }
die() { say "oan install: $*" >&2; exit 1; }

case "$(uname -s)" in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-musl ;;
  *) die "oan runs on macOS and Linux here (on Windows, use install.ps1)" ;;
esac
case "$(uname -m)" in
  arm64 | aarch64) arch=aarch64 ;;
  x86_64 | amd64) arch=x86_64 ;;
  *) die "unsupported CPU: $(uname -m)" ;;
esac
target="$arch-$os"

if [ -n "${OAN_DOWNLOAD_BASE:-}" ]; then
  base="$OAN_DOWNLOAD_BASE"
elif [ "$version" = latest ]; then
  base="https://github.com/$repo/releases/latest/download"
else
  base="https://github.com/$repo/releases/download/$version"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Downloading oan for $target..."
curl -fsSL "$base/oan-$target.tar.gz" -o "$tmp/oan.tar.gz" || die "download failed ($base/oan-$target.tar.gz)"
curl -fsSL "$base/oan-$target.tar.gz.sha256" -o "$tmp/sum" || die "checksum download failed; not installing"
want="$(cut -d' ' -f1 < "$tmp/sum")"
if command -v shasum >/dev/null 2>&1; then
  got="$(shasum -a 256 "$tmp/oan.tar.gz" | cut -d' ' -f1)"
else
  got="$(sha256sum "$tmp/oan.tar.gz" | cut -d' ' -f1)"
fi
[ -n "$want" ] && [ "$want" = "$got" ] || die "checksum mismatch; not installing"
tar -xzf "$tmp/oan.tar.gz" -C "$tmp"
"$tmp/oan" --version >/dev/null 2>&1 || die "the downloaded binary doesn't run here; not installing"
# macOS knows a program by its code signature when it asks whether it may reach the local network. A build signed by
# a developer team keeps that answer across updates; an unsigned one is a new program every time. So a signed
# oan is only ever replaced by one its team signed.
team() { codesign -dv "$1" 2>&1 | sed -n 's/^TeamIdentifier=//p'; }
if [ "$os" = apple-darwin ] && [ -e "$bin_dir/oan" ] && command -v codesign >/dev/null 2>&1; then
  have="$(team "$bin_dir/oan")"
  case "$have" in
    "" | "not set") ;;
    *) [ "$(team "$tmp/oan")" = "$have" ] ||
         die "this build isn't signed by $have, as the installed one is; keeping that one (try again later)" ;;
  esac
fi
mkdir -p "$bin_dir"
# A new file moved into place: whatever runs the old one (a hook, an agent's tools) never sees a half-written binary.
install -m 755 "$tmp/oan" "$bin_dir/.oan.new"
mv -f "$bin_dir/.oan.new" "$bin_dir/oan"
say "Installed $bin_dir/oan ($("$bin_dir/oan" --version))"

case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) say "Note: $bin_dir is not on your PATH. Add this to your shell profile:"
     say "  export PATH=\"$bin_dir:\$PATH\"" ;;
esac

rm -rf "$tmp"
trap - EXIT INT TERM

# Hand over to oan. It asks a few questions in a terminal. Under
# `curl | sh` our stdin is the script, so read answers from the terminal itself when there is one.
# (In a subshell: in dash, Ubuntu's sh, a failed redirection on `:` would end the whole script.)
if [ -z "${OAN_NO_PROMPT:-}" ] && (exec </dev/tty) 2>/dev/null; then
  case "${1:-}" in
    "") exec "$bin_dir/oan" </dev/tty ;;
    -*|init|join|setup) exec "$bin_dir/oan" "$@" </dev/tty ;;
    *) exec "$bin_dir/oan" join "$@" </dev/tty ;;
  esac
fi
case "${1:-}" in
  "") say ""; say "Next, in a terminal: oan" ;;
  -*|init|join|setup) exec "$bin_dir/oan" "$@" ;;
  *) exec "$bin_dir/oan" join "$@" ;;
esac
