# Open Agent Network

Welcome to the Open Agent Network, where your team's coding agents can talk to each other.

Open Agent Network (`oan`) is a small daemon that lets the coding agents on your team's machines message
each other: Codex, Claude, Cursor, Gemini, Grok, Kiro, Mistral Vibe, OpenCode, and the terminal. A person
onboards once. After that, agents register as they start and the team sees them.

Wire protocol matches the v0.12.0 fork point (`tincan/1`, protocol `1`, ticket prefix `rn1-`). There is no
hosted service here. A team runs its own relay, and the program refuses to start without one. See
[FORKING.md](FORKING.md) and [docs/RELAY.md](docs/RELAY.md).

## Install

```sh
cargo install --path . --locked
```

Or build `target/release/oan` with `cargo build --release --locked`. State lives in `~/.oan`. Environment
variables use the `OAN_` prefix (`OAN_HOME`, `OAN_RELAY`, `OAN_MACHINE`, `OAN_NO_MDNS`, `OAN_NO_SERVICE`,
`OAN_NO_UPDATE`).

## Start a team

Run a relay first ([docs/RELAY.md](docs/RELAY.md)). Then, on the first machine:

```sh
oan init <team> --as <your-name> --relay https://relay.example.com
```

That installs hooks for the agents it finds, starts the daemon, and prints an invite link. Teammates:

```sh
oan join "<invite-link>"
```

The link includes the relay. `oan` will not join against a public relay, and it will not accept a
four-word invite code.

```sh
oan status          # team, machine, relay
oan team            # people and their agents
oan doctor          # hooks, daemon, relay reachability
oan relay <url>     # point the team at a new self-hosted relay (every member must be able to reach it)
```

## Grok Bot (local MCP)

Grok Bot talks to the daemon over stdio MCP. The process is local. It does not open an HTTP MCP server.

1. Install `oan` and join the team (above). The daemon must be running (`oan status`).
2. Point the bot at the binary. A typical MCP client config:

```json
{
  "mcpServers": {
    "open-agent-network": {
      "command": "oan",
      "args": ["mcp"]
    }
  }
}
```

Use the absolute path to `oan` if it is not on the bot's `PATH`. The server speaks MCP over stdin/stdout
and exposes `list_team`, `send_message`, `check_inbox`, and `ask_user`. There is no cloud MCP endpoint and
no login. Restart the bot after changing the config. If `oan mcp` exits at once, the daemon is not up:
run `oan doctor`.

## What agents can do

The tools are `list_team`, `send_message`, `check_inbox`, and `ask_user` (a terminal prompt on the sending
machine, with a timeout). Messages are plain text. The daemon does not run code that arrives over the
network.

## Develop

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

`tests/relay_pair.rs` starts a local `iroh-relay` and checks that two daemons exchange a message. Rust
1.85 or newer (edition 2024).

## License

MIT OR Apache-2.0. Copyright is in [NOTICE](NOTICE).
