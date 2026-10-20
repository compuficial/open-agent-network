#!/bin/sh
# Local stand-in for Railway: the relay image on plain HTTP, Caddy terminating TLS,
# and two debug `oan` daemons using the https:// URL. Not run by CI.
set -eu
cd "$(dirname "$0")/../.."

relay_port=18080
proxy_port=18443
image=oan-iroh-relay:smoke
work="$(mktemp -d)"
caddy_pid=""

cleanup() {
  if [ -n "$caddy_pid" ]; then
    kill "$caddy_pid" 2>/dev/null || true
  fi
  sudo docker rm -f oan-relay-smoke >/dev/null 2>&1 || true
  if [ -n "${alice:-}" ] && [ -x target/debug/oan ]; then
    OAN_HOME="$alice" OAN_NO_PROMPT=1 target/debug/oan stop >/dev/null 2>&1 || true
  fi
  if [ -n "${bob:-}" ] && [ -x target/debug/oan ]; then
    OAN_HOME="$bob" OAN_NO_PROMPT=1 target/debug/oan stop >/dev/null 2>&1 || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

sudo docker build -t "$image" deploy/railway
sudo docker rm -f oan-relay-smoke >/dev/null 2>&1 || true
sudo docker run -d --name oan-relay-smoke -e PORT=8080 -p "127.0.0.1:${relay_port}:8080" "$image" >/dev/null

i=0
while [ "$i" -lt 30 ]; do
  if curl -sf "http://127.0.0.1:${relay_port}/healthz" | grep -q '"status":"ok"'; then
    break
  fi
  i=$((i + 1))
  sleep 1
done
curl -sf "http://127.0.0.1:${relay_port}/healthz" | grep -q '"status":"ok"'

openssl req -x509 -newkey rsa:2048 -keyout "$work/ca.key" -out "$work/ca.pem" -days 1 -nodes -subj "/CN=oan-relay-ca" >/dev/null 2>&1
openssl req -newkey rsa:2048 -keyout "$work/cert.key" -out "$work/cert.csr" -nodes -subj "/CN=127.0.0.1" -addext "subjectAltName=IP:127.0.0.1" >/dev/null 2>&1
openssl x509 -req -in "$work/cert.csr" -CA "$work/ca.pem" -CAkey "$work/ca.key" -CAcreateserial -out "$work/cert.pem" -days 1 -copy_extensions copy >/dev/null 2>&1

cat >"$work/Caddyfile" <<EOF
{
  auto_https off
  admin off
}
https://127.0.0.1:${proxy_port} {
  tls ${work}/cert.pem ${work}/cert.key
  reverse_proxy 127.0.0.1:${relay_port}
}
EOF

caddy="${CADDY:-caddy}"
if [ ! -x "$caddy" ]; then
  caddy=/tmp/caddy
fi
"$caddy" run --config "$work/Caddyfile" >"$work/caddy.log" 2>&1 &
caddy_pid=$!

i=0
while [ "$i" -lt 20 ]; do
  if curl -sf --cacert "$work/ca.pem" "https://127.0.0.1:${proxy_port}/healthz" | grep -q '"status":"ok"'; then
    break
  fi
  i=$((i + 1))
  sleep 0.5
done
curl -sf --cacert "$work/ca.pem" "https://127.0.0.1:${proxy_port}/healthz" | grep -q '"status":"ok"'

cargo build --bin oan
oan="$PWD/target/debug/oan"
alice="$work/alice"
bob="$work/bob"
mkdir -p "$alice/unix-home" "$bob/unix-home"
relay_url="https://127.0.0.1:${proxy_port}"

common() {
  home="$1"
  machine="$2"
  shift 2
  OAN_HOME="$home" \
    OAN_MACHINE="$machine" \
    OAN_RELAY_CA="$work/ca.pem" \
    OAN_NO_MDNS=1 \
    OAN_NO_SERVICE=1 \
    OAN_NO_UPDATE=1 \
    OAN_NO_TRAY=1 \
    OAN_NO_PROMPT=1 \
    HOME="$home/unix-home" \
    "$oan" "$@"
}

init_out="$(common "$alice" alice-box init localnet --as alice --relay "$relay_url" --no-setup)"
printf '%s\n' "$init_out"
printf '%s\n' "$init_out" | grep -q 'rn1-'
link="$(printf '%s\n' "$init_out" | tr ' ' '\n' | grep 'rn1-' | head -n 1 | tr -d '"')"

join_out="$(common "$bob" bob-box join "$link" --as bob --no-setup)"
printf '%s\n' "$join_out"
printf '%s\n' "$join_out" | grep -q 'Joined team localnet as bob'

common "$bob" bob-box listen --as terminal --wait 30 >"$work/heard" 2>&1 &
listen_pid=$!

i=0
while [ "$i" -lt 40 ]; do
  if common "$alice" alice-box who --json | grep -q terminal; then
    break
  fi
  i=$((i + 1))
  sleep 0.5
done
common "$alice" alice-box who --json | grep -q terminal
common "$alice" alice-box send bob/terminal ping-from-alice

i=0
while [ "$i" -lt 40 ]; do
  if grep -q ping-from-alice "$work/heard"; then
    break
  fi
  i=$((i + 1))
  sleep 0.5
done
grep -q ping-from-alice "$work/heard"
wait "$listen_pid"

echo "daemons reached ${relay_url}"
