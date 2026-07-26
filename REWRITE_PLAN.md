# BoxPacker Rewrite Plan

Status: Milestone M1 is complete and Milestone M2 is in progress. The
compatibility shell, exact geometry boundary, safe report, saved-solution
adapter, and immutable baseline metrics pass `devenv test`; the domain solver
interface, objective, deadlines, metrics, and independent solution validator
are implemented; the event-based constructive baseline is independently valid
and improves the saved fixture, both candidate libraries are isolated behind
the backend boundary, and the broader bake-off fixtures are next.

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

- Current milestone: `M2 — solver bake-off` (`IN PROGRESS`).
- Milestone M0 status: `DONE`; `devenv test` passed on both target platforms.
- Milestone M1 status: `DONE`; the compatibility fixture passes its exact
  baseline regression and renders through the safe report template.
- Git boundary: `boxpacker/` is its own Git repository on the `main` branch,
  independent from the old reference repository. Its parent is not a Git
  repository.
- Completed before this handoff: `M0`, all of `M1`, and `M2.1` through
  `M2.2`. The compatibility shell, exact independent validator, backend
  contract, provisional objective, and clean-room constructive baseline are
  present. No old solver or scoring code was copied.
- Completed in this handoff: `M2.3`.
  - Exact evaluation versions `bin-packing = 0.3.0` and
    `u-nesting-d3 = 0.6.0` are locked. Default parallel features are disabled,
    keeping thread behavior bounded and the dependency experiment portable.
    This is an evaluation pin, not a D-005 selection.
  - `BinPackingBackend` maps heterogeneous one-use container inventory and
    stable item IDs into the dependency's integer multi-bin model, translates
    its vertical/depth axes back to BoxPacker axes, preclassifies individually
    infeasible items, and reports the dependency's 32,768-unit axis cap without
    truncation. Dependency types do not cross the adapter.
  - `UNestingBackend` compensates for the dependency's single-boundary API by
    visiting each heterogeneous container once in deterministic order and
    solving only remaining stable item instances. It passes scaled integers as
    exactly representable `f64`, accepts only integral returned coordinates,
    applies the dependency's orientation index locally, and independently
    validates the combined result.
  - Only deterministic `u-nesting-d3` strategies are exposed because its
    randomized strategies have no seed in the shared 0.6 configuration.
    Sequential per-container assignment and absent explored-state metrics are
    recorded model-fit limitations for `M2.5`.
  - `bin-packing` has native heterogeneous inventory and seed support but no
    deadline option. Its adapter exposes contact-point, basic extreme-point,
    and auto strategies without leaking dependency enums.
  - Both adapters pass heterogeneous inventory, rotation, no-fit, stable-ID,
    and exact independent validation tests. On the current fixture,
    `bin-packing` contact-point places 41 items / 535,042.896 volume and
    sequential `u-nesting-d3` extreme-point places 49 / 568,460.714; both trail
    the clean-room 53 / 587,815.524 result, so neither candidate is selected.
- Verification passed:
  - host-authorized `devenv test` passed on ARM64 macOS with the locked
    environment, including formatting, Clippy with warnings denied, all tests,
    and the debug build;
  - `devenv shell -- cargo test --all-targets --all-features` reported 48
    passing tests (5 unit and 43 integration), with no failures;
  - `devenv shell -- cargo clippy --all-targets --all-features -- -D warnings`
    passed;
  - `devenv shell -- cargo test --test candidate_adapters` reported 4 passing
    cross-adapter tests;
  - `git diff --check` passed;
  - the old project's status was unchanged after implementation.
- Development-test note: the first full test/Clippy run after replacing
  temporary metric printing put the expected-value tuple on the wrong test
  loop, producing two missing-variable and two unused-variable diagnostics.
  The tuple was moved to the current-fixture loop; the focused suite and final
  full checks then passed.
- Environment note: host-authorized devenv was used because the prior handoff
  established that the sandbox cannot open the user Nix cache or connect to
  `/nix/var/nix/daemon-socket/socket` (`Operation not permitted`). No portable
  project configuration was changed. Cargo network access downloaded the two
  exact candidate versions and their locked transitive dependencies.
- Prior cross-platform evidence remains valid: `devenv test` passed on native
  ARM64 macOS and in an isolated x86-64 QEMU/Colima Linux guest using the same
  lock. Automated dual-platform CI remains `M4.4`.
- Principal verification commands used in this handoff:

  ```sh
  devenv test
  devenv shell -- cargo test --test candidate_adapters
  devenv shell -- cargo test --all-targets --all-features
  devenv shell -- cargo clippy --all-targets --all-features -- -D warnings
  git diff --check
  git -C ../oldBoxPackerForDeletion status --short --branch
  ```
- Changed files in this handoff: `Cargo.lock`, `Cargo.toml`, `README.md`,
  `REWRITE_PLAN.md`, `src/solver/bin_packing.rs`, `src/solver/mod.rs`,
  `src/solver/u_nesting.rs`, and `tests/candidate_adapters.rs`.
- Old-project state rechecked before and after implementation:
  `M src/main.rs`, `?? output.html`, and `?? output.old.html`. All three remain
  user-owned and unchanged by this handoff.
- Solver dependencies selected: none. Section 5.1's bake-off is mandatory.
- Decision register changes: none.
- Remaining blockers: none for `M2.4`. Automated dual-platform CI remains
  planned as `M4.4`.
- Exact next task: `M2.4`, add small known-answer, adversarial, current,
  input-permutation, and generated 8-container/77-item fixtures. Run every
  viable backend through independent validation and record objective values;
  keep deadline-performance judgments for `M2.5`.
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
| D-005 | OPEN | Select solver libraries only after the M2 correctness/quality bake-off. |
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

### M2 — solver bake-off (`IN PROGRESS`)

- [x] `M2.1` Implement the domain-level `SolverBackend` interface, objective,
  deadlines, metrics, and independent solution validator.
- [x] `M2.2` Implement the event-based constructive baseline.
- [x] `M2.3` Adapt each viable library candidate without leaking its types past
  the backend boundary.
- [ ] `M2.4` Add small known-answer, adversarial, current, permutation, and 8/77
  fixtures.
- [ ] `M2.5` Benchmark correctness, objective quality, runtime, determinism,
  portability, maintenance, and licensing.
- [ ] `M2.6` Record an ADR selecting or rejecting each dependency and pin the
  selected versions.

Exit: selected solver is demonstrably valid, portable, and better than the old
saved result or has a documented gap and next experiment.

### M3 — anytime improvement engine (`TODO`)

- [ ] `M3.1` Add deterministic portfolio work partitioning and seeded search.
- [ ] `M3.2` Add deadline cancellation and a validated shared incumbent.
- [ ] `M3.3` Add move, swap, rotation, ejection-chain, and ruin/recreate
  neighborhoods.
- [ ] `M3.4` Add bounded branch-and-bound repair for small residuals.
- [ ] `M3.5` Emit structured progress/metrics without coupling UI code to
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
