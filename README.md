# BoxPacker

This is the clean-room Rust rewrite of the project in
`../oldBoxPackerForDeletion`.

This directory is an independent Git repository. The old project beside it has
separate history and remains read-only reference material.

The rewrite is intentionally only a buildable scaffold for now. The proposed
implementation and acceptance criteria are in
[`REWRITE_PLAN.md`](REWRITE_PLAN.md).

`REWRITE_PLAN.md` is also the canonical AI handoff record. Continuing agents
must update its current snapshot, task checkboxes, decision register, and
verification evidence before ending a handoff.
Repository-level agent instructions in [`AGENTS.md`](AGENTS.md) direct future
agents to that record automatically.

## Development environment

The native development environment is managed by
[devenv](https://devenv.sh/):

```sh
devenv shell
cargo run
```

Run the scaffold quality checks with:

```sh
devenv test
```

The configuration uses the stable Rust toolchain and has no
architecture-specific packages, so the same files are intended to work on
both `aarch64-darwin` and `x86_64-linux`.
