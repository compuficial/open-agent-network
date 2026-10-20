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

# Per-client receive cap. iroh-relay enforces this; connection-accept limits in that file do nothing.
rx="${OAN_RELAY_RX_BYTES_PER_SEC:-1048576}"
burst="${OAN_RELAY_RX_BURST_BYTES:-4194304}"
case "$rx" in
  '' | *[!0-9]*)
    echo "OAN_RELAY_RX_BYTES_PER_SEC must be a positive integer, got '${rx}'" >&2
    exit 1
    ;;
esac
case "$burst" in
  '' | *[!0-9]*)
    echo "OAN_RELAY_RX_BURST_BYTES must be a positive integer, got '${burst}'" >&2
    exit 1
    ;;
esac
if [ "$rx" -lt 1 ] || [ "$burst" -lt 1 ]; then
  echo "rate limit values must be at least 1 byte" >&2
  exit 1
fi

cfg="$(mktemp)"
cat >"$cfg" <<EOF
enable_relay = true
enable_quic_addr_discovery = false
enable_metrics = false
http_bind_addr = "0.0.0.0:${port}"

[limits.client.rx]
bytes_per_second = ${rx}
max_burst_bytes = ${burst}
EOF

exec iroh-relay --config-path "$cfg"
