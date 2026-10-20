# Forking and updates

Open Agent Network tracks [reason-machines/reason-network](https://github.com/reason-machines/reason-network).
The fork point is tag `v0.12.0`.

## What is ours

Keep these on rebase. They are the whole policy change:

- `src/fork.rs` brands the binary, state directory, and environment prefix, and is the only place that
  names hosts this build refuses.
- `src/net.rs` builds the endpoint with `presets::Minimal` and one self-hosted relay. `DEFAULT_RELAYS` is
  empty. Nothing calls `presets::N0` or fetches a relay list.
- `src/relays.rs` does not fetch. `src/proof.rs` trusts no built-in issuer key. `OAN_TRUST_KEYS` is for
  tests.
- `src/team.rs` still decodes `rn1-` tickets and rejects the compact public-relay marker (`0xFF`).
- `src/daemon/teamops.rs` requires a self-hosted relay on init, join, and `set_relay`, and refuses
  four-word invite codes.
- `src/main.rs` reports an unmanaged build (`managed()` is false), so this binary never downloads an update.
- Docs: `README.md`, `docs/RELAY.md`, `FORKING.md`, `NOTICE`.

## What stays compatible

Do not rename these. Peers on the same relay still speak the upstream wire protocol.

- ALPN `tincan/1` and `tincan/join/1` (`src/net.rs`)
- protocol version `1`
- ticket prefix `rn1-` (and the legacy `tincan1` spelling, decoded only)
- the signed roster, chat, and file-offer payloads

## Rebase

```sh
git remote add upstream https://github.com/reason-machines/reason-network.git
git fetch upstream --tags
git rebase --onto upstream/v0.12.0 <previous-upstream-tag> cursor/open-agent-network-fork-775d
```

Expect conflicts in the files listed above, in `Cargo.toml` (package name and the `oan` binary), and in
`setup.rs` (agent list and hook text). After each rebase:

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

`tests/branding.rs` fails the build if a user-facing string grows back a forbidden host or the old product
name. `tests/relay_pair.rs` fails if two local daemons cannot pass a message through a local relay.

## Hosts this build will not use

`src/fork.rs` rejects relay URLs under `reasonmachines.com`, `n0.iroh.link`, and `n0.computer`, and the
bare word `default`. The endpoint never enables public address discovery, so it does not publish to or
resolve through the public DNS those networks use.
