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

Railway's public edge terminates TLS and forwards TCP. It does not hand you a public UDP port, so do not
publish UDP 7842 there and expect the internet to reach it.

1. Deploy `iroh-relay` listening on the `PORT` Railway sets (HTTP, behind the edge).
2. Generate a Railway domain. The public URL is `https://<name>.up.railway.app`.
3. Pass that HTTPS URL to `oan init --relay` and `oan relay`.

Peers will relay over TCP through the edge. That is the supported Railway shape. A VM is the better
default when you can open UDP.

## Local

`oan` accepts `http://127.0.0.1` and RFC1918 addresses for a lab. `tests/relay_pair.rs` starts
`iroh-relay` in `--dev` mode on a loopback port and runs two daemons against it.
