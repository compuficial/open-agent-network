# Add a machine to the team

Someone on the team already ran `oan init` and sent you a line that looks like this:

```
Join Open Agent Network: rn1-…
```

You need the `oan` program, then that line. You do not need an account.

## Install

From a checkout of this repository:

```sh
cargo install --path . --locked
```

Or download a release binary and put `oan` on your `PATH`. Check with `oan --version`.

## Join

Paste the whole invite line:

```sh
oan join "rn1-…"
```

The link includes the team's relay. If `oan` asks for your name, use the one your teammates expect (`--as <name>` skips the question).

When it finishes, `oan status` shows the team and `oan doctor` checks the daemon and the relay. Restart any coding agent you already had open so it loads the tools.

## If it will not join

- The link must start with `rn1-`. A four-word code is refused.
- You must reach the same relay the founder used. `oan doctor` says when the relay does not answer.
- A name already in the team is taken. Pick another with `--as`.

Messages stay on the machines in the team. The relay forwards encrypted traffic and cannot read it.
