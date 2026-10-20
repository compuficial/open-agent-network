# Run a relay

Open Agent Network will not start a team without a relay. By default that relay is one you run.
`oan init --community` opts into the public list instead. The relay forwards encrypted QUIC. It cannot
read messages. Every teammate must reach the same relay, or the same set when more than one is configured.

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

Cap how much one client can send. `iroh-relay` enforces `[limits.client.rx]` (a token bucket on bytes
read from each client). Its connection-accept limits are not implemented, so leave them unset. Put
this in the file you pass as `--config-path`, next to the bind addresses:

```toml
[limits.client.rx]
bytes_per_second = 1048576
max_burst_bytes = 4194304
```

That is 1 MiB/s sustained and a 4 MiB burst. Raise it if the team moves larger files through the relay.

Limit who can connect, if you want, with `iroh-relay`'s access controls to the roster's endpoint ids
(`oan team --json` lists them). The URL must be `https://` in production. `oan` refuses hostnames under
`reasonmachines.com`, `n0.iroh.link`, and `n0.computer`.

## Railway (HTTPS only)

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/open-agent-network-relay)

That button deploys the container in [deploy/railway](../deploy/railway). The public edge terminates TLS
on 443 and forwards plain HTTP to the container's `PORT`. WebSocket connections, which is how iroh relays
when it cannot use UDP, stay open. There is no inbound UDP, so the image does not advertise QUIC address
discovery and does not bind UDP 7842.

The image builds `iroh-relay` at the version locked next to `iroh` in `Cargo.lock` (1.3.0). It listens
on `0.0.0.0:$PORT`, with metrics and QUIC turned off. `GET /healthz` returns JSON when the process is up.

The template sets the service. This table is the source of truth:

| Setting | Value |
| --- | --- |
| Root directory | `/deploy/railway` |
| Dockerfile path | `Dockerfile` |
| Healthcheck path | `/healthz` |
| Restart policy | `ON_FAILURE` |
| Public port | 443 at Railway's edge. Do not publish UDP. |
| Container port | The `PORT` Railway injects. The process binds it. Leave `PORT` unset in the variables UI. |
| Required variables | None. `PORT` is injected. `RUST_LOG` is optional and only changes `iroh-relay` logs. |
| Rate limit | On. `OAN_RELAY_RX_BYTES_PER_SEC` (default `1048576`) and `OAN_RELAY_RX_BURST_BYTES` (default `4194304`). |

`railway.json` and `railway.toml` in that directory still describe the image, and the tests read them.
Railway did not pick those files up when they were set as the config-file path. Railway is moving that
configuration to `.railway/railway.ts`.

Generate a Railway domain. The URL teammates use is `https://<name>.up.railway.app` (no port):

```sh
oan init my-team --as alice --relay https://<name>.up.railway.app
```

Peers who cannot reach each other directly relay over that HTTPS connection. It is slower than a VM
that also publishes UDP 7842, and the relay still cannot read messages. A direct QUIC path is still
used when the two machines can open one.

`deploy/railway/smoke.sh` runs this shape on the local machine: the image behind a TLS-terminating
proxy, and two `oan` daemons using the `https://` URL. It is not part of CI.

## Local

`oan` accepts `http://127.0.0.1` and RFC1918 addresses for a lab. `tests/relay_pair.rs` starts
`iroh-relay` in `--dev` mode on a loopback port and runs two daemons against it.

## Host a community relay

A community relay is a normal relay whose URL is listed in `relays.json` at the root of this
repository. Teams do not use that list unless someone passes `--community`. The default is still a
relay the team runs itself.

1. Deploy one. The Railway template is
   [https://railway.com/deploy/open-agent-network-relay](https://railway.com/deploy/open-agent-network-relay).
   A VM from the section above is fine too. The process must answer `GET /healthz` with HTTP 200 on
   the same `https://` origin teammates will use.
2. Leave the per-client receive limit on. The Railway image sets 1 MiB/s and a 4 MiB burst. Override
   those with `OAN_RELAY_RX_BYTES_PER_SEC` and `OAN_RELAY_RX_BURST_BYTES` if you need to. The VM
   config above is the same limit.
3. Open a pull request that adds one object to the `relays` array. Copy the fields from
   `relays.schema.json`: `url`, `operator`, `github` (your handle), `region`, `added` (`YYYY-MM-DD`),
   and optional `notes`. The URL has to be `https://`. Hosts this project refuses (`reasonmachines.com`,
   `n0.iroh.link`, `n0.computer`, and other `iroh.link` names) are rejected here too.
4. CI fetches `GET /healthz` for every URL the pull request adds. A failure blocks the merge. After
   it is on `main`, a nightly job checks every entry. Three failed nights in a row open a pull request
   that removes that entry.

What you can see: source IP addresses, iroh endpoint ids, and when connections open and close. What
you cannot see: message contents. The relay forwards ciphertext.

Someone who wants this list runs:

```sh
oan init my-team --as alice --community
oan relay --community
```

`oan` downloads `relays.json` from the raw file on `main` (or `OAN_COMMUNITY_RELAYS_URL`), skips
anything that fails the same host checks, times `GET /healthz`, and keeps the fastest three. Those
three are the team's relay set, so iroh can fail over from one to another. The command prints the
operator and GitHub handle of each one before it saves them. An empty list is an error. `--relay`
and `--community` cannot be combined.
