#!/bin/sh
# Point this machine at a self-hosted Open Agent Network relay and start or join a team.
#
#   OAN_RELAY=https://relay.example.com ./setup.sh init <team> --as <name>
#   ./setup.sh join 'rn1-…' --as <name>
#
# Requires `oan` on PATH (or OAN_BIN). Refuses to run without a self-hosted relay on init.
set -eu

bin="${OAN_BIN:-oan}"
command -v "$bin" >/dev/null 2>&1 || {
  printf 'oan setup: %s is not on PATH. Build it with: cargo install --path . --locked\n' "$bin" >&2
  exit 1
}

if [ "${1:-}" = "init" ]; then
  if [ -z "${OAN_RELAY:-}" ]; then
    printf 'oan setup: set OAN_RELAY to your relay (https://relay.example.com)\n' >&2
    exit 1
  fi
  shift
  exec "$bin" init "$@" --relay "$OAN_RELAY"
fi

if [ "${1:-}" = "join" ]; then
  shift
  exec "$bin" join "$@"
fi

printf 'usage: setup.sh init <team> --as <name>    (needs OAN_RELAY)\n' >&2
printf '       setup.sh join <rn1-link> --as <name>\n' >&2
exit 1
