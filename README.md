# BoxPacker

This is the clean-room Rust rewrite of the project in
`../oldBoxPackerForDeletion`.

This directory is an independent Git repository. The old project beside it has
separate history and remains read-only reference material.

The compatibility-shell milestone is complete. The legacy JSON input/output
DTOs and command-line contract are present, compatibility dimensions are
converted once to exact scaled-integer geometry, and the HTML report is a safe
static template backed by serialized JSON data. A temporary saved-solution
adapter maps the current fixture back to stable input IDs and exact AABBs
without certifying solver geometry, and exact regression assertions preserve
its baseline quality metrics. The rewrite is now implementing the domain solver
and bake-off. Backend-neutral solutions, deadlines, common metrics, provisional
lexicographic objectives, and independent exact validation are present; the
clean-room event-based constructive backend now produces a deterministic,
independently valid result that improves the saved fixture baseline. Candidate
library adapters are isolated behind the same backend trait, but neither has
been selected. Known-answer, adversarial, current, reversed-current, and
generated 8-container/77-item fixtures now record independently validated
objectives. The M2.5 bake-off now records release runtime, determinism,
deadline behavior, portability, dependency cost, maintenance, and licensing in
[`docs/bakeoff/M2.5.md`](docs/bakeoff/M2.5.md); solver selection remains
reserved for the M2.6 ADR.
The proposed implementation and acceptance criteria are in
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
