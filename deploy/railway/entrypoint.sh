#!/bin/sh
# Railway terminates TLS at the edge and forwards plain HTTP to $PORT.
# There is no public UDP, so QUIC address discovery stays off.
set -eu

port="${PORT:-8080}"
case "$port" in
  '' | *[!0-9]*)
    echo "PORT must be an integer, got '${port}'" >&2
    exit 1
    ;;
esac
if [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
  echo "PORT ${port} is outside 1-65535" >&2
  exit 1
fi

cfg="$(mktemp)"
cat >"$cfg" <<EOF
enable_relay = true
enable_quic_addr_discovery = false
enable_metrics = false
http_bind_addr = "0.0.0.0:${port}"
EOF

exec iroh-relay --config-path "$cfg"
