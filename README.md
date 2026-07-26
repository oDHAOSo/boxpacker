# BoxPacker

This is the clean-room Rust rewrite of the project in
`../oldBoxPackerForDeletion`.

This directory is an independent Git repository. The old project beside it has
separate history and remains read-only reference material.

The compatibility-shell and solver bake-off milestones are complete. The
legacy JSON input/output DTOs and command-line contract are present,
compatibility dimensions are
converted once to exact scaled-integer geometry, and the HTML report is a safe
static template backed by serialized JSON data. A temporary saved-solution
adapter maps the current fixture back to stable input IDs and exact AABBs
without certifying solver geometry, and exact regression assertions preserve
its baseline quality metrics. The rewrite now has a selected domain solver.
Backend-neutral solutions, deadlines, common metrics, volume-first
lexicographic objectives, and independent exact validation are present; the
clean-room event-based constructive backend now produces a deterministic,
independently valid result that improves the saved fixture baseline. Candidate
library adapters were evaluated behind the same backend trait and removed
after neither justified its production integration cost. Known-answer,
adversarial, current, reversed-current, and
generated 8-container/77-item fixtures now record independently validated
objectives. The M2.5 bake-off now records release runtime, determinism,
deadline behavior, portability, dependency cost, maintenance, and licensing in
[`docs/bakeoff/M2.5.md`](docs/bakeoff/M2.5.md); the accepted selection and
candidate rejections are in
[`docs/decisions/0001-solver-backend.md`](docs/decisions/0001-solver-backend.md).
M3 now has deterministic seeded portfolio work partitioning whose stable
reduction retains the canonical constructor as a quality floor across thread
counts. Solve requests now carry a cloneable cooperative-cancellation token;
the portfolio establishes a canonical incumbent, independently validates every
completed worker candidate before publishing it to the shared store, and stops
launching work when cancelled or out of time. Improvement neighborhoods are
now present as deterministic move, swap, forced-rotation, three-item
ejection-chain, and ruin/recreate construction-plan transformations. Every
transformed plan rebuilds through exact geometry and must pass the shared
validator before it can improve the incumbent. Bounded exact repair for small
residuals now freezes the validated incumbent placements, branches over
place/unplaced choices and every unique rotation at maximal-space face events,
and prunes on unattainable volume/count bounds. Its exhaustion flag applies
only to that bounded residual subproblem; overall solver status remains
heuristic. Callers can now attach a thread-safe progress sink to a solve
request and receive typed portfolio, stable work-unit, validated-objective,
repair, and final aggregate-metric events without importing CLI or report
types. M3's exit proofs cover monotonic objective quality across fixed effort
levels, input-order reversal, solution and non-time metric reproducibility, and
normalized concurrent progress. Connecting the selected portfolio to
compatible output is now complete: the CLI reads and validates the legacy
input, runs the selected portfolio, independently validates its incumbent, and
writes both the legacy JSON shape and the safe HTML report. Fast, balanced, and
thorough currently use fixed monotonic work budgets of 1, 8, and 14 units with
default time limits of 1, 10, and 30 seconds; `--time-limit` overrides only the
deadline. Malformed JSON now retains line/column context and identifies the
exact DTO field path for shape errors; exact dimension validation still
reports all bad fields together. Report rendering rejects invalid numeric data
and template placeholder mistakes before producing HTML while retaining its
script-safe JSON escaping. Bounded properties now cover varied exact
rotations, exhaustive cell preservation across randomized free-space splits,
and randomized valid/invalid solution validation. An isolated nightly
`cargo-fuzz` package contains seeded raw-JSON and small selected-portfolio
targets without changing the stable production toolchain. Criterion records
solve-plus-independent-validation performance for every production preset on
both representative fixtures. A GitHub Actions matrix now runs the locked
stable format, strict Clippy, all-target test, and release-build gates on
native ARM64 macOS and x86-64 Linux runners. Algorithm/status semantics,
reproducibility limits, presets, and status meanings are documented in
[`docs/usage.md`](docs/usage.md).
All planned rewrite implementation tasks are complete, and the product has
confirmed the volume-first objective. The first hosted run of the checked-in
native ARM64 macOS/x86-64 Linux workflow passed, completing M4.
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
