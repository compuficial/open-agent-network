# Run a relay

Open Agent Network will not start a team without a relay you control. The relay forwards encrypted QUIC.
It cannot read messages. Every teammate must reach the same URL.

## What to open

| Path | Ports | Notes |
| --- | --- | --- |
| A VM (recommended) | TCP 80 and 443, UDP 7842 | HTTPS on 443, HTTP-01 on 80, QUIC on UDP 7842 |
| Railway | TCP 443 only, via the edge | Fine for NAT traversal. No public UDP. See below. |

QUIC (UDP 7842) is the fast path. TCP 443 is enough for peers who cannot speak UDP to each other: iroh
falls back to relaying over HTTPS. A relay that is HTTPS-only still works. It is slower and keeps a TCP
connection per peer on the relay.

## VM

```sh
# iroh-relay 1.3.x, the same line as the `iroh` crate in Cargo.lock
iroh-relay \
  --dev=false \
  --http-bind-addr 0.0.0.0:80 \
  --https-bind-addr 0.0.0.0:443 \
  --quic-bind-addr 0.0.0.0:7842 \
  --hostname relay.example.com \
  --cert-path /etc/oan/fullchain.pem \
  --key-path /etc/oan/privkey.pem
```

Point DNS `relay.example.com` at the VM. Issue the certificate with Let's Encrypt (HTTP-01 needs port 80).
Then:

```sh
oan init my-team --as alice --relay https://relay.example.com
```

Limit who can connect, if you want, with `iroh-relay`'s access controls to the roster's endpoint ids
(`oan team --json` lists them). The URL must be `https://` in production. `oan` refuses hostnames under
`reasonmachines.com`, `n0.iroh.link`, and `n0.computer`.

## Railway (HTTPS only)

Railway runs the container in [deploy/railway](../deploy/railway). The public edge terminates TLS on 443
and forwards plain HTTP to the container's `PORT`. WebSocket connections, which is how iroh relays when
it cannot use UDP, stay open. There is no inbound UDP, so the image does not advertise QUIC address
discovery and does not bind UDP 7842.

The image builds `iroh-relay` at the version locked next to `iroh` in `Cargo.lock` (1.3.0). It listens
on `0.0.0.0:$PORT`, with metrics and QUIC turned off. `GET /healthz` returns JSON when the process is up.
`railway.json` and `railway.toml` in that directory are the same service settings.

Service settings for the template:

| Setting | Value |
| --- | --- |
| Root directory | `deploy/railway` |
| Dockerfile path | `Dockerfile` |
| Config file | `railway.json` or `railway.toml` (same settings) |
| Healthcheck path | `/healthz` |
| Healthcheck timeout | 120 seconds |
| Public port | 443 at Railway's edge. Do not publish UDP. |
| Container port | The `PORT` Railway injects. The process binds it. Leave `PORT` unset in the variables UI. |
| Required variables | None. `PORT` is injected. `RUST_LOG` is optional and only changes `iroh-relay` logs. |

Generate a Railway domain. The URL teammates use is `https://<name>.up.railway.app` (no port):

```sh
oan init my-team --as alice --relay https://<name>.up.railway.app
```

The README has a Deploy on Railway button. Its link is a placeholder until the template exists
(`<!-- TEMPLATE_URL -->`).

Peers who cannot reach each other directly relay over that HTTPS connection. It is slower than a VM
that also publishes UDP 7842, and the relay still cannot read messages. A direct QUIC path is still
used when the two machines can open one.

`deploy/railway/smoke.sh` runs this shape on the local machine: the image behind a TLS-terminating
proxy, and two `oan` daemons using the `https://` URL. It is not part of CI.

## Local

`oan` accepts `http://127.0.0.1` and RFC1918 addresses for a lab. `tests/relay_pair.rs` starts
`iroh-relay` in `--dev` mode on a loopback port and runs two daemons against it.
