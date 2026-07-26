# BoxPacker Rewrite Plan

Status: Milestones M0 through M2 are complete and Milestone M3 is in progress.
The compatibility shell, exact geometry boundary, safe report, saved-solution
adapter, and immutable baseline metrics pass `devenv test`; the domain solver
interface, objective, deadlines, metrics, and independent solution validator
are implemented; the event-based constructive baseline is independently valid
and improves the saved fixture; both candidate libraries were isolated,
evaluated, and removed; and the full M2.5 correctness, quality, runtime,
determinism, deadline, portability, maintenance, and licensing evidence is
recorded. ADR 0001 selects the in-tree clean-room backend and rejects both
external candidates. Deterministic seeded portfolio work partitioning is
implemented with cooperative deadline cancellation and a validated shared
incumbent. Deterministic move, swap, rotation, ejection-chain, and
ruin/recreate reconstruction neighborhoods are implemented; bounded exact
event repair for small residuals is implemented; structured progress and
metrics are implemented without UI coupling; monotonic effort and
reproducibility proofs are next.

Reference implementation: `../oldBoxPackerForDeletion` (read-only during the
rewrite).

## 0. AI handoff protocol

This file is the canonical execution plan and handoff record. An AI agent
continuing the rewrite should be able to resume from this file plus the
repository state without relying on chat history.

### Start-of-handoff checklist

1. Read this entire file before changing code.
2. Inspect `git status` in this project and in
   `../oldBoxPackerForDeletion`. Never discard or modify pre-existing changes.
3. Treat the old project as read-only reference material. Do not port its
   packing/scoring implementation.
4. Start with the first unchecked task in the current milestone unless the
   handoff snapshot names a blocker or a different next task.
5. Re-read the applicable milestone exit criteria before implementation.
6. Keep I/O compatibility, geometry, solver, independent validation, and
   reporting separated as described in Section 4.

Use these status labels in this document: `TODO`, `IN PROGRESS`, `BLOCKED`, and
`DONE`. Only one milestone should be `IN PROGRESS`. A task is `DONE` only when
its tests and relevant milestone evidence exist.

### End-of-handoff checklist

Before handing work to another agent:

1. Update the snapshot below with the date, current milestone, completed work,
   changed files, commands run and their outcomes, blockers, and the exact next
   task.
2. Update the milestone checkboxes and decision table. Do not describe
   unverified work as complete.
3. Run `devenv test`; if host policy prevents it, record the exact failure and
   run the closest pinned-toolchain checks available.
4. Preserve failing tests and benchmark evidence when they express unfinished
   requirements; do not delete them to make a handoff green.
5. Record material design/dependency choices in `docs/decisions/` using a short
   ADR before coupling more code to the choice.
6. Keep commits scoped to one task ID when this project has a Git repository.
   Never commit changes inside the old reference repository.

### Current handoff snapshot — 2026-07-26

- Current milestone: `M3 — anytime improvement engine` (`IN PROGRESS`).
- Milestone M0 status: `DONE`; `devenv test` passed on both target platforms.
- Milestone M1 status: `DONE`; the compatibility fixture passes its exact
  baseline regression and renders through the safe report template.
- Git boundary: `boxpacker/` is its own Git repository on the `main` branch,
  independent from the old reference repository. Its parent is not a Git
  repository.
- Completed before this handoff: `M0`, all of `M1`, and `M2.1` through
  `M2.4`. The compatibility shell, exact validator, clean-room baseline,
  isolated adapters for both exact-version dependency candidates, and the
  fixed correctness/quality matrix are present. No old solver or scoring code
  was copied and no dependency is selected.
- Completed in this handoff: every task from `M2.5` through `M3.5`.
  - `tests/fixtures/generated/scale_8x77.json` is a clean-room fixed-scale
    document with eight heterogeneous containers, 77 uniquely named items,
    724,566.920 item volume, and 882,287.290 capacity. Its README records its
    deterministic eleven-profile/seven-repeat construction.
  - A small known-answer fixture fills one container with four items for every
    backend. An adversarial 5×5×1 case proves that volume slack alone cannot fit
    two 3×4×1 items; every backend correctly places one.
  - The current fixture and a complete reversal of both input arrays produce
    independently valid, identical `ObjectiveValue`s for all three backends.
    Recorded current results remain clean-room 53 / 587,815.524,
    `bin-packing` 41 / 535,042.896, and `u-nesting-d3` 49 / 568,460.714.
  - The 8/77 fixture records exact objective components:
    clean-room `(73, 694,614.920, 8 used, 199.220 unsupported,
    836,936.280 bounding)`, `bin-packing`
    `(70, 687,975.920, 8, 362.560, 807,771.800)`, and `u-nesting-d3`
    `(73, 694,614.920, 8, 282.900, 830,038.000)`.
  - Every recorded candidate passes the independent exact validator before its
    metrics are asserted.
  - Criterion release benchmarks on native ARM64 macOS record the clean-room,
    `bin-packing`, and `u-nesting-d3` point estimates as 577.17 µs, 42.831 ms,
    and 286.24 µs on the current fixture, and 851.11 µs, 182.70 ms, and
    440.12 µs on 8/77.
  - Fixed seed/effort reproduces identical domain solutions for every backend.
    The clean-room and `u-nesting-d3` adapters return independently valid 8/77
    incumbents within a 5 ms deadline plus a 250 ms shutdown allowance.
    `bin-packing` has no deadline/cancellation API and is explicitly excluded
    from that deadline assertion.
  - `docs/bakeoff/M2.5.md` records model fit, enabled dependency graphs,
    maintenance and MIT licensing evidence, native ARM64 release portability,
    and the remaining lack of post-candidate x86-64 verification.
  - ADR 0001 selects the in-tree clean-room constructive backend. It rejects
    `bin-packing` 0.3.0 for its measured quality/runtime loss, objective and
    dimension mismatch, and absent deadline API. It rejects `u-nesting-d3`
    0.6.0 because its speed does not improve the primary objective enough to
    justify a single-boundary `f64` conversion layer and metric/seed gaps.
  - Both rejected adapters and solver dependencies were removed. The
    `SolverBackend` seam and exact-pinned development-only Criterion benchmark
    remain. The selected production solver has no external version to pin.
  - M2's exit criterion is met: the selected solver is independently valid,
    portable without a native solver dependency, and improves the old saved
    packed volume from 582,885.612 to 587,815.524.
  - `PortfolioBackend` creates a fixed deterministic plan containing the
    canonical constructor plus seeded item-order searches, assigns work units
    round-robin within the requested thread bound, joins local results, sorts
    them by stable work index, independently validates every candidate, and
    reduces them with `ObjectiveValue`.
  - The same fixed seed and eight work units return an identical current-fixture
    solution with one or four threads. Because work unit zero is canonical, the
    selected portfolio objective retains or improves the selected M2 baseline.
  - `SolveRequest` now owns a cloneable `CancellationToken`; constructive
    search polls cancellation and its monotonic deadline at item and candidate
    boundaries. The portfolio always produces and validates its canonical
    incumbent before optional seeded workers.
  - Each completed worker candidate passes the independent exact validator
    before entering the mutex-protected shared incumbent. Publication compares
    the domain objective with a stable work-index tie-break, while final
    metrics are reduced in work-index order so scheduling cannot change them.
  - Seeded workers stop cooperatively between work units and within
    constructive enumeration. A pre-cancelled request returns a valid complete
    canonical incumbent, and a 10,000-work-unit 8/77 request honors a 5 ms
    deadline within the 250 ms shutdown allowance.
  - `ConstructionPlan` separates deterministic item order and forced exact
    rotations from geometry construction. `improve.rs` adds seeded move, swap,
    forced-rotation, three-item ejection-chain, and ruin/recreate
    transformations; each preserves the complete stable-ID item permutation.
  - The default eight-work-unit portfolio runs the canonical constructor, two
    seeded shuffles, and all five neighborhood kinds. Each reconstruction uses
    the same exact maximal-space engine and independently validated incumbent
    publication path. Current and 8/77 tests prove the neighborhood portfolio
    retains or improves the canonical objective.
  - `exact.rs` adds bounded branch-and-bound repair for at most six residual
    items and 512 nodes by default. It freezes the independently validated
    incumbent placements, branches on place/unplaced choices, enumerates every
    unique rotation at current maximal-space face events, and prunes branches
    whose remaining volume/count bound cannot improve the incumbent.
  - Repair polls the shared deadline/cancellation request, returns the original
    valid incumbent if bounded before finding an improvement, and exposes an
    `exhaustive` flag explicitly limited to the frozen-placement event
    subproblem. The portfolio retains `Heuristic` overall status and publishes
    repair through the same validator as a ninth candidate.
  - `SolveRequest::with_progress_sink` accepts a thread-safe domain
    `ProgressSink`. The portfolio emits typed start, work-start,
    independently-validated objective, repair completion, and final aggregate
    metric events. Work events carry stable indices and kinds so concurrent
    delivery can be normalized without relying on scheduling order.
  - Progress types depend only on solver/domain objective and metric types; no
    CLI, report, serialization, or UI type enters solver orchestration.
- Verification passed:
  - host-authorized `devenv test` passed on ARM64 macOS with the locked
    environment, including formatting, Clippy with warnings denied, all tests,
    and the debug build;
  - before M2.6 cleanup,
    `devenv shell -- cargo test --all-targets --all-features` reported 54
    passing tests (5 unit and 49 integration), with no failures;
  - after M2.6 cleanup, the same command reported 50 passing tests (5 unit and
    45 integration), with no failures; the four removed tests exercised only
    the rejected adapters and their evidence remains in the M2.5 commit;
  - `devenv shell -- cargo clippy --all-targets --all-features -- -D warnings`
    passed;
  - `devenv shell -- cargo test --test bakeoff_fixtures` reported 4 passing
    cross-backend fixture-matrix tests;
  - `devenv shell -- cargo test --test bakeoff_evaluation` reported 2 passing
    determinism/deadline tests;
  - `devenv shell -- cargo bench --bench current_fixture -- --noplot`
    completed all six release benchmark groups;
  - `devenv shell -- cargo build --release` passed on native ARM64 macOS;
  - the post-selection release build, strict Clippy run, and final
    `devenv test` gate passed with no solver dependency in the production
    graph;
  - after M3.1,
    `devenv shell -- cargo test --all-targets --all-features` reported 54
    passing tests (7 unit and 47 integration), with no failures;
  - `devenv shell -- cargo test --test portfolio` reported 2 passing
    reproducibility and quality-floor tests;
  - the M3.1 strict Clippy run, release build, and `devenv test` gate passed;
  - the four-thread Criterion point estimates were 1.8906 ms for the
    eight-work-unit current-fixture portfolio and 2.5999 ms for its 8/77
    portfolio;
  - after M3.2,
    `devenv shell -- cargo test --all-targets --all-features` reported 58
    passing tests (9 unit and 49 integration), with no failures;
  - `devenv shell -- cargo test --test portfolio` reported 4 passing
    reproducibility, quality-floor, cancellation, and deadline tests;
  - the M3.2 strict Clippy run and release build passed;
  - after M3.3,
    `devenv shell -- cargo test --all-targets --all-features` reported 62
    passing tests (12 unit and 50 integration), with no failures;
  - `devenv shell -- cargo test --lib solver::improve` reported 3 passing
    deterministic-permutation and forced-orientation tests;
  - `devenv shell -- cargo test --test portfolio` reported 5 passing tests,
    including current and 8/77 objective quality floors;
  - the M3.3 strict Clippy run and release build passed;
  - the four-thread neighborhood Criterion point estimates were 2.1675 ms on
    the current fixture and 3.1145 ms on 8/77;
  - after M3.4,
    `devenv shell -- cargo test --all-targets --all-features` reported 65
    passing tests (15 unit and 50 integration), with no failures;
  - `devenv shell -- cargo test --lib solver::exact` reported 3 passing
    exhaustive, bounded-status, and item-bound tests;
  - `devenv shell -- cargo test --test portfolio` reported 5 passing tests with
    nine validated candidates when repair is eligible;
  - the M3.4 strict Clippy run and release build passed;
  - the four-thread repaired-portfolio Criterion point estimates were
    2.4627 ms on the current fixture and 3.3434 ms on 8/77;
  - after M3.5,
    `devenv shell -- cargo test --all-targets --all-features` reported 66
    passing tests (15 unit and 51 integration), with no failures;
  - `devenv shell -- cargo test --test progress` reported 1 passing typed-event
    and stable-work-identifier test;
  - the M3.5 strict Clippy run and release build passed;
  - `git diff --check` passed;
  - the old project's status was unchanged after implementation.
- Development-test note: sandboxed devenv attempts for the M2.5 production
  dependency-tree and release checks failed exactly with `Failed to create GC
  root` because the Nix daemon socket returned `Operation not permitted`.
  Host-authorized locked-devenv reruns were used; no portable configuration was
  weakened.
- M2.6 development-test note: the first strict Clippy run identified a
  single-element loop left by reducing a deadline matrix to the selected
  backend. The test was simplified, then formatting, strict Clippy, and
  `devenv test` all passed.
- Environment note: host-authorized devenv was used because the prior handoff
  established that the sandbox cannot open the user Nix cache or connect to
  `/nix/var/nix/daemon-socket/socket` (`Operation not permitted`). No portable
  project configuration was changed. Cargo network access downloaded the two
  exact candidate versions and their locked transitive dependencies.
- Prior cross-platform evidence remains valid for the pre-candidate project:
  `devenv test` passed on native ARM64 macOS and in an isolated x86-64
  QEMU/Colima Linux guest. Candidate dependencies were added after that run;
  automated dual-platform verification remains `M4.4`.
- Principal verification commands used in this handoff:

  ```sh
  devenv test
  devenv shell -- cargo test --test bakeoff_fixtures
  devenv shell -- cargo test --test bakeoff_evaluation
  devenv shell -- cargo test --test portfolio
  devenv shell -- cargo test --all-targets --all-features
  devenv shell -- cargo clippy --all-targets --all-features -- -D warnings
  devenv shell -- cargo bench --bench current_fixture -- --noplot
  devenv shell -- cargo build --release
  devenv shell -- cargo tree -p bin-packing -e normal
  devenv shell -- cargo tree -p u-nesting-d3 -e normal
  git diff --check
  git -C ../oldBoxPackerForDeletion status --short --branch
  ```
- Changed files in M2.5: `Cargo.lock`, `Cargo.toml`, `README.md`,
  `REWRITE_PLAN.md`, `benches/current_fixture.rs`, `docs/bakeoff/M2.5.md`, and
  `tests/bakeoff_evaluation.rs`.
- Changed files in M2.6: `Cargo.lock`, `Cargo.toml`, `README.md`,
  `REWRITE_PLAN.md`, `benches/current_fixture.rs`,
  `docs/bakeoff/M2.5.md`, `docs/decisions/0001-solver-backend.md`,
  `src/solver/mod.rs`, `tests/bakeoff_evaluation.rs`, and
  `tests/bakeoff_fixtures.rs`; removed `src/solver/bin_packing.rs`,
  `src/solver/u_nesting.rs`, and `tests/candidate_adapters.rs`.
- Changed files in M3.1: `README.md`, `REWRITE_PLAN.md`,
  `benches/current_fixture.rs`, `src/solver/constructive.rs`,
  `src/solver/mod.rs`, `src/solver/portfolio.rs`, and `tests/portfolio.rs`.
- Changed files in M3.2: `README.md`, `REWRITE_PLAN.md`,
  `src/solver/constructive.rs`, `src/solver/mod.rs`,
  `src/solver/portfolio.rs`, and `tests/portfolio.rs`.
- Changed files in M3.3: `README.md`, `REWRITE_PLAN.md`,
  `src/solver/constructive.rs`, `src/solver/improve.rs`, `src/solver/mod.rs`,
  `src/solver/portfolio.rs`, and `tests/portfolio.rs`.
- Changed files in M3.4: `README.md`, `REWRITE_PLAN.md`,
  `src/solver/constructive.rs`, `src/solver/exact.rs`, `src/solver/mod.rs`,
  `src/solver/portfolio.rs`, and `tests/portfolio.rs`.
- Changed files in M3.5: `README.md`, `REWRITE_PLAN.md`, `src/solver/mod.rs`,
  `src/solver/portfolio.rs`, and `tests/progress.rs`.
- Old-project state rechecked before and after implementation:
  `M src/main.rs`, `?? output.html`, and `?? output.old.html`. All three remain
  user-owned and unchanged by this handoff.
- Solver dependencies selected: none. The in-tree clean-room constructive
  backend is selected by ADR 0001; both evaluated dependencies are rejected.
- Decision register changes: D-005 is now `LOCKED`.
- Remaining blockers: none for `M3.6`. Automated dual-platform CI remains
  planned as `M4.4`.
- Exact next task: `M3.6`, prove that increased fixed effort retains or
  improves the incumbent and that fixed effort, seed, and thread settings
  reproduce solutions, aggregate metrics other than elapsed time, and
  normalized structured progress.
- Open user/product decision: whether packed volume or packed item count is the
  first tie-break when not all items can fit. Continue with the provisional
  volume-first objective until the decision is made; the objective type must
  make changing this ordering local.

### Decision register

| ID | Status | Decision |
| --- | --- | --- |
| D-001 | LOCKED | Preserve the existing input shape and initial report behavior. |
| D-002 | LOCKED | Use a clean-room solver; old packing and weighted scoring code are reference-only. |
| D-003 | LOCKED | Use checked scaled-integer geometry; no raster step size or floating-point collision logic. |
| D-004 | PROVISIONAL | Rank unplaced volume before unplaced item count; awaiting product confirmation. |
| D-005 | LOCKED | Select the in-tree clean-room backend; reject and remove both evaluated solver dependencies per ADR 0001. |
| D-006 | DEFERRED | Add native MILP/CP-SAT only if measured quality justifies its portability cost. |
| D-007 | LOCKED | Keep `boxpacker/` as its own Git repository on `main`, separate from the old reference repository. |

## 1. Objective and boundaries

Replace the packing logic completely while preserving the parts of the current
program that are already useful:

- Keep the current JSON input shape: top-level `containers` and `contents`
  arrays, with `name`, `width`, `length`, and `height` fields.
- Keep the command-line defaults (`input.json`, `output.json`) and generate the
  HTML report beside the JSON output.
- Initially preserve the report's information and interaction model. Report and
  input-format enhancements are separate follow-up work.
- Use devenv as the only documented development environment. It must work
  natively on ARM64 macOS and x86-64 Linux.
- Optimize packing quality under a user-visible time budget. Do not claim a
  global optimum unless an exact search proves it.

The current fixture contains 6 containers and 57 items. Its total item volume
is 608,981.542 against 735,033.29 of container capacity (82.85% theoretical
utilization if everything fits). The saved result places 49 items with
582,885.612 volume (79.30% utilization) and leaves 8 items out. The expected
near-term size is about 8 containers and 77 items.

## 2. Why the old solver will not be carried forward

The current implementation:

- enumerates every `(x, y, z)` point on a fixed 0.5-unit grid;
- greedily commits each item to the first container that accepts it;
- tries only six rotations at each grid point;
- samples ten combinations of hand-written item/container ordering rules;
- uses floating-point geometry and approximate rotation deduplication;
- mixes packed volume and squared utilization with an arbitrary `1,000,000`
  weight, even though unplaced items are not directly part of the score; and
- performs a linear collision scan for every candidate point.

That design explains the step-size sensitivity, order sensitivity, poor search
coverage, and scaling problem. None of that placement or scoring code should be
ported.

## 3. Target behavior and objective

Use an explicit lexicographic objective instead of a weighted formula:

1. reject geometrically invalid solutions;
2. minimize unplaced item volume;
3. minimize unplaced item count;
4. minimize the number of used containers;
5. maximize compactness/support and deterministic tie-break values.

This makes the priority of each goal inspectable and prevents a tuning constant
from silently changing what "better" means. Before the solver is finalized,
confirm whether item count or item volume should take precedence when not
everything can fit. The proposed default is volume because the stated goal is
utilization. A later input-format revision can add per-item priority without
changing the solver architecture.

Every run must accept:

- `--time-limit` for an anytime search;
- `--seed` for reproducible randomized work;
- `--threads` for bounded parallelism; and
- a fast/balanced/thorough preset that only changes search effort, never
  geometry correctness.

## 4. Architecture

Keep I/O, geometry, solving, validation, and reporting separate:

```text
src/
  main.rs              CLI composition and exit codes
  cli.rs               arguments and presets
  model.rs             compatibility input/output DTOs
  geometry.rs          scaled integer dimensions, rotations, AABBs
  objective.rs         lexicographic solution ordering
  validate.rs          independent input and placement validation
  solver/
    mod.rs             backend trait, deadlines, metrics
    portfolio.rs       parallel anytime orchestration
    constructive.rs    extreme-point/maximal-space placement
    improve.rs         local and large-neighborhood improvement
    exact.rs           bounded exact repair/proof for small residuals
  report/
    mod.rs             report view model
    template.html      preserved Three.js report
tests/
  compatibility.rs
  geometry_properties.rs
  solution_properties.rs
  quality_regression.rs
benches/
  current_fixture.rs
```

The compatibility structs will deserialize the existing JSON unchanged.
Internally, dimensions will be converted once to scaled positive integers.
For the existing one-decimal data, one internal unit is 0.1 input units.
Conversion must reject non-finite, non-positive, or over-precision values with
field-specific errors. All bounds, overlap checks, contact tests, volumes, and
candidate coordinates then use checked integer arithmetic; there is no epsilon
and no spatial step size.

Items receive stable internal IDs so duplicate names remain legal. Output maps
IDs back to the original names and dimensions.

## 5. Solver strategy

### 5.1 Dependency evaluation spike

Start by adapting and benchmarking current Rust 3D packing libraries rather
than committing to one from its feature list:

- `bin-packing` 0.3 exposes multi-bin integer models, six rotations, extreme
  point variants, 3D guillotine beam search, layer/wall/column builders,
  GRASP/local search, Rayon parallelism, and a restricted exact backend.
- `u-nesting-d3` 0.6 exposes extreme-point, GA, BRKGA, and simulated-annealing
  solvers plus spatial indexing and stability checks, but its fit for
  heterogeneous multi-container inventory needs validation.

Put candidates behind an internal `SolverBackend` trait. Run them on the
current fixture and generated adversarial cases, independently validate every
placement, compare quality/runtime, inspect maintenance/licensing, and retain
only a backend that is correct and materially useful. These libraries are new;
the rewrite must remain able to replace one without touching I/O or reporting.

### 5.2 Constructive placement

The required fallback/core backend uses event-based placement:

- maintain extreme points and/or maximal empty cuboids created by container
  walls and already placed item faces;
- enumerate only unique legal rotations and candidate event positions;
- prune dominated free spaces and candidates;
- rank candidates using residual space, wall/item contact, fragmentation,
  support, and resulting bounding extent; and
- use a spatial index for collision and support queries if profiling justifies
  it.

Candidate coordinates are derived from geometry, not a raster grid. Therefore
the result cannot change because a 0.5 constant became 0.4.

### 5.3 Global search

Run a portfolio of complementary constructors in parallel. Feed the strongest
solutions into an adaptive large-neighborhood search:

- change item-to-container assignments;
- change rotations and placement order;
- perform move, swap, and ejection-chain repairs;
- "ruin" congested regions or weakly utilized containers and reconstruct them;
- prioritize moves using failure history rather than fixed hand-tuned order
  constants; and
- share the best validated incumbent through a thread-safe solution store.

The process is anytime: it produces a deterministic baseline quickly, improves
until the deadline, then returns the best validated incumbent. A fixed seed,
thread count, and effort budget must reproduce the same result. Quality should
not depend on the input array order; a regression test will permute both arrays
and compare objective values.

### 5.4 Exact escalation and honest optimality

Full exact 3D packing for roughly 77 items is generally not a reasonable
interactive promise. Use exact methods where they are effective:

- dimensional and volume lower bounds before search;
- branch-and-bound for small instances;
- bounded exact repair for the final unplaced items plus a small ejection
  neighborhood; and
- the library's restricted exact backend only when its preconditions hold.

Report `heuristic`, `bound_matched`, or `proven_optimal` status. "Best found
within 30 seconds" and "optimal" must never be conflated.

## 6. Output and report compatibility

First migrate the current report into a template with the same:

- container and global utilization metrics;
- Three.js box rendering and labels;
- item selection/highlighting;
- unplaced-item list; and
- x-ray/wireframe toggle.

Keep the existing JSON result fields (`containers`, nested `placed_items`, and
`unplaced_items`). Additional solver metadata should be additive and optional:
algorithm, seed, elapsed time, objective components, explored candidates, and
optimality status.

Serialize report data as JSON rather than interpolating raw names into
JavaScript/HTML. The report generator must escape script-ending sequences and
handle arbitrary item names safely. Visual redesign, offline bundling of
Three.js, and input-schema enhancements belong to a later milestone.

## 7. Verification and benchmarks

### Correctness gates

- validate positive finite dimensions and unique internal IDs;
- verify every placement is inside its original container;
- verify every placed orientation is a permutation of the input dimensions;
- verify pairwise non-overlap using exact integer comparisons;
- verify each input item appears exactly once in placed or unplaced output;
- property-test rotations, free-space splitting, and solution validation; and
- fuzz JSON parsing and small randomized instances.

The validator must be independent from the solver so a solver defect cannot
certify its own output.

### Quality and performance gates

- record the old fixture baseline: 49/57 placed, 582,885.612 packed volume,
  79.30% total-container utilization;
- require the new deterministic fast baseline to be valid and no worse than
  that saved result;
- require balanced/thorough search to be monotonic: it may keep or improve the
  incumbent, never replace it with a worse result;
- add a generated 8-container/77-item scale fixture and enforce the configured
  deadline with a small shutdown allowance;
- run permutation-invariance checks over item and container input order; and
- use Criterion/profiling to optimize measured hotspots rather than introduce
  geometry-resolution constants.

CI must run formatting, Clippy with warnings denied, tests, and release builds
on both `aarch64-darwin` and `x86_64-linux`. Quality benchmarks should be
recorded separately from noisy wall-clock CI checks.

## 8. Implementation milestones

### M0 — scaffold and environment (`DONE`)

- [x] `M0.1` Create this standalone Cargo project beside the old project.
- [x] `M0.2` Add the architecture-neutral devenv Rust environment and lock.
- [x] `M0.3` Pass format, Clippy, tests, and a debug build with the pinned
  toolchain.
- [x] `M0.4` Initialize this folder as its own Git repository on `main` and make
  a baseline commit without touching the old repository.
- [x] `M0.5` Pass `devenv test` on ARM64 macOS without the current host's Nix
  trust-policy failure.
- [x] `M0.6` Pass `devenv test` on x86-64 Linux.

Exit: the scaffold checks pass through devenv on both target platforms, or
platform verification is delegated to an explicitly recorded CI task with the
local host-policy limitation documented.

### M1 — compatibility shell and baseline (`DONE`)

- [x] `M1.1` Copy the current input and saved output into test fixtures; do not
  modify the originals or import old solver code.
- [x] `M1.2` Port only the input/output DTOs and CLI contract.
- [x] `M1.3` Add exact integer conversion and field-specific input validation.
- [x] `M1.4` Port the current HTML report into a safe template.
- [x] `M1.5` Add a temporary adapter that can read the known saved solution for
  report and validator testing.
- [x] `M1.6` Record fixture baseline metrics in an asserted regression test.

Exit: existing input produces compatible JSON/HTML from a validated fixture.

### M2 — solver bake-off (`DONE`)

- [x] `M2.1` Implement the domain-level `SolverBackend` interface, objective,
  deadlines, metrics, and independent solution validator.
- [x] `M2.2` Implement the event-based constructive baseline.
- [x] `M2.3` Adapt each viable library candidate without leaking its types past
  the backend boundary.
- [x] `M2.4` Add small known-answer, adversarial, current, permutation, and 8/77
  fixtures.
- [x] `M2.5` Benchmark correctness, objective quality, runtime, determinism,
  portability, maintenance, and licensing.
- [x] `M2.6` Record an ADR selecting or rejecting each dependency and pin the
  selected versions.

Exit: selected solver is demonstrably valid, portable, and better than the old
saved result or has a documented gap and next experiment.

### M3 — anytime improvement engine (`IN PROGRESS`)

- [x] `M3.1` Add deterministic portfolio work partitioning and seeded search.
- [x] `M3.2` Add deadline cancellation and a validated shared incumbent.
- [x] `M3.3` Add move, swap, rotation, ejection-chain, and ruin/recreate
  neighborhoods.
- [x] `M3.4` Add bounded branch-and-bound repair for small residuals.
- [x] `M3.5` Emit structured progress/metrics without coupling UI code to
  solver logic.
- [ ] `M3.6` Prove through tests that increased effort retains or improves the
  incumbent and that fixed effort/seed/thread settings reproduce results.

Exit: longer presets monotonically improve or retain quality, input
permutations preserve objective quality, and deadline behavior is bounded.

### M4 — integration and hardening (`TODO`)

- [ ] `M4.1` Connect the selected solver to compatible JSON and HTML outputs.
- [ ] `M4.2` Add malformed-input diagnostics and safe report serialization.
- [ ] `M4.3` Complete property tests, fuzz targets, and release benchmarks.
- [ ] `M4.4` Add dual-platform CI for ARM64 macOS and x86-64 Linux.
- [ ] `M4.5` Document algorithm/status semantics, presets, reproducibility, and
  optimality claims.
- [ ] `M4.6` Update the handoff snapshot with final evidence and remove
  obsolete provisional decisions.

Exit: `devenv test` passes on both target platforms, the current fixture beats
or matches the recorded baseline, and the 8/77 scale fixture completes within
its configured budget.

## 9. Decisions deliberately deferred

- Item priorities, orientation restrictions, weight, fragility, and stacking
  constraints require input-schema changes and should follow compatibility.
- Changing the visual design or input shape is out of scope for the logic
  rewrite.
- A native MILP/CP-SAT dependency should not be introduced unless the bake-off
  proves enough quality benefit to justify cross-platform packaging cost.

## 10. Technical references

- devenv Rust environment:
  <https://devenv.sh/languages/rust/>
- `bin-packing` 3D solver API:
  <https://docs.rs/bin-packing/0.3.0/bin_packing/three_d/>
- `bin-packing` source and algorithm catalog:
  <https://github.com/doublesharp/bin-packing>
- `u-nesting-d3` solver API:
  <https://docs.rs/u-nesting-d3/0.6.0/u_nesting_d3/>
- Heßler, Hintsch, and Wienkamp, extreme-point randomized greedy search for a
  real multiple-bin-size 3D problem:
  <https://arxiv.org/abs/2410.01445>
