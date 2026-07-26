# BoxPacker

This is the clean-room Rust rewrite of the project in
`../oldBoxPackerForDeletion`.

This directory is an independent Git repository. The old project beside it has
separate history and remains read-only reference material.

The rewrite is currently implementing its compatibility shell. The legacy JSON
input/output DTOs and command-line contract are present, and compatibility
input dimensions are validated and converted once to exact scaled-integer
geometry. The compatibility HTML report is also available as a safe static
template backed by serialized JSON report data. A temporary saved-solution
adapter maps the current compatibility fixture back to stable input IDs and
exact AABBs without certifying solver geometry. Fixture baseline assertions and
solver work remain milestone tasks. The proposed implementation and acceptance
criteria are in
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
cargo run -- --help
```

Run the project quality checks with:

```sh
devenv test
```

The configuration uses the stable Rust toolchain and has no
architecture-specific packages, so the same files are intended to work on
both `aarch64-darwin` and `x86_64-linux`.
