# ADR 0001: select the clean-room constructive solver

Status: accepted

Date: 2026-07-26

## Context

M2 required a measured bake-off before selecting a solver dependency. Three
backends were isolated behind `SolverBackend` and evaluated with independent
exact validation:

- the in-tree clean-room maximal-space/face-event constructor;
- `bin-packing` 0.3.0 using its contact-point extreme-points strategy; and
- `u-nesting-d3` 0.6.0 using its sequential extreme-point strategy.

The complete fixture results, Criterion intervals, deadline behavior,
dependency graphs, portability limits, maintenance snapshot, and licensing
evidence are retained in [`../bakeoff/M2.5.md`](../bakeoff/M2.5.md) and in the
M2.5 commit. D-004's volume-first objective is now locked and remains the
documented objective against which this decision was made.

## Decision

Select the in-tree clean-room constructive backend as BoxPacker's core solver.
It is the only evaluated backend retained in the production tree. It has no
external solver version to pin.

Reject and remove both evaluated solver dependencies and their adapters:

- Reject `bin-packing` 0.3.0. It placed less volume than both the clean-room
  backend and the saved legacy result on the current fixture, was the slowest
  candidate on both measured fixtures, internally prioritizes item count
  instead of D-004 packed volume, caps dimensions below BoxPacker's domain, and
  exposes no 3D deadline/cancellation API.
- Reject `u-nesting-d3` 0.6.0. It was the fastest candidate and tied the
  clean-room backend's primary placed-count/volume result on 8/77, but did not
  beat the saved legacy volume on the current fixture. Its one-boundary API
  prevents joint heterogeneous-inventory optimization, its `f64` geometry
  creates an unnecessary conversion boundary, and its tested path provides
  neither a consumed seed nor compatible explored-state metrics.

Criterion 0.8.2 remains exact-pinned as development-only benchmark
infrastructure. It is not a solver and does not enter the production binary
dependency graph.

## Rationale

The selected backend:

- passes the independent exact validator on known-answer, adversarial,
  current, reversed-current, and 8/77 fixtures;
- produces 53/57 placed and 587,815.524 packed volume on the current fixture,
  improving the saved result's 582,885.612 packed volume;
- ties the strongest external primary result on 8/77 and has lower unsupported
  area;
- directly uses BoxPacker's checked integer geometry, heterogeneous inventory,
  objective, deadline, solution, and metrics types; and
- solves the scale fixture in under one millisecond at the recorded release
  benchmark point estimate, leaving ample headroom for M3's anytime search.

The external adapters therefore add maintenance and model-conversion cost
without a measured quality capability that the selected core needs.

## Consequences

- M3 will build deterministic portfolio and anytime improvement logic around
  the domain backend and constructive primitives rather than an external
  solver API.
- The `SolverBackend` boundary remains, so a future dependency can be evaluated
  without changing compatibility I/O, exact validation, objective ordering, or
  reporting.
- The M2.5 document is historical evidence. Its reproduction commands for
  rejected candidates require checking out the M2.5 commit.
- No selected candidate dependency needs post-bake-off x86 verification.
  Locked dual-platform project CI remains M4.4.
- D-006 remains deferred: a native MILP/CP-SAT dependency is justified only by
  future measured quality evidence.

## Reconsider when

Reopen this decision only when a candidate demonstrates an independently valid
solution that materially improves the domain objective or supplies a missing
proof/search capability worth its portability and integration cost. Any new
candidate must repeat the M2 evidence matrix before entering production.
