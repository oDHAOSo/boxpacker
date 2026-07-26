# BoxPacker usage and solver semantics

## Running the rewrite

The command reads the legacy input shape, writes compatible JSON to the
requested output path, and writes the HTML report beside it by replacing the
output extension with `.html`.

```sh
devenv shell -- cargo run --release -- \
  --input input.json \
  --output output.json \
  --preset balanced \
  --seed 0 \
  --threads 4
```

Use `cargo run -- --help` for the complete CLI. Dimensions must be finite,
positive, and expressible with at most one decimal place. Coordinates and
collision checks use exact scaled integers after this input boundary.

The legacy JSON output retains `containers`, nested `placed_items`, and
`unplaced_items`. Solver metadata is not embedded in that compatibility
document. The CLI prints the placed count and honest status; callers of the
library can also inspect `RunSummary`.

## Objective

Every candidate is rejected unless the independent exact validator confirms
bounds, orientation, non-overlap, and exactly-once item coverage. Valid
candidates are ordered lexicographically:

1. minimize unplaced volume;
2. minimize unplaced item count;
3. minimize used container count;
4. minimize unsupported area;
5. minimize occupied bounding volume; and
6. use stable geometry and ID values to break remaining ties.

The first two priorities are deliberately not blended into a weighted score.
Volume-before-count remains provisional pending product confirmation. Changing
that order is local to the objective type.

## Search algorithm

The selected backend is an in-tree clean-room deterministic portfolio:

- The canonical constructor orders items deterministically and places them at
  maximal-free-space face events. It tries every unique axis-aligned rotation
  with checked integer geometry.
- Optional work units derive deterministic seeds and cycle through a shuffled
  construction plus move, swap, forced-rotation, three-item ejection-chain,
  and ruin/recreate reconstruction neighborhoods.
- A bounded residual repair may run when the canonical incumbent has at most
  six unplaced items. It freezes existing placements and explores at most 512
  nodes over exact face events.
- Every completed candidate passes the independent validator before it can
  enter the shared incumbent. Concurrent results reduce by objective and
  stable work index, not completion order.

The repair's `exhaustive` flag applies only to its frozen-placement residual
subproblem. It does not prove that the full packing problem is optimal.

## Presets and deadlines

| Preset | Construction work units | Default deadline |
| --- | ---: | ---: |
| `fast` | 1 | 1 second |
| `balanced` | 8 | 10 seconds |
| `thorough` | 14 | 30 seconds |

The work count includes the canonical construction. Eligible bounded residual
repair is additional and uses the same deadline. More construction work is a
deterministic candidate superset: tests prove objective monotonicity at 1, 2,
4, 8, and 14 units on both representative fixtures.

`--time-limit SECONDS` replaces only the preset deadline. It never changes
geometry resolution or validity. `--threads COUNT` bounds parallel workers;
the solver may use fewer workers when fewer optional work units exist.

Deadline and cancellation polling occurs between item/candidate boundaries.
The solver always returns a complete, independently valid incumbent, possibly
with remaining items marked unplaced if the deadline is already exhausted.

## Status meanings

| Status | Meaning |
| --- | --- |
| `heuristic` | Best independently valid incumbent found by the configured work and deadline; no global optimality claim. |
| `bound_matched` | Reserved for a future backend whose incumbent matches a valid global bound without a complete proof. |
| `proven_optimal` | Reserved for a future backend that exhaustively proves global optimality for the stated objective. |

The current portfolio always reports `heuristic`, including when bounded
residual repair exhausts its restricted subproblem.

## Reproducibility limits

- With the same validated input, preset/work count, seed, and thread bound,
  runs reproduce the same solution and non-time metrics when the wall-clock
  deadline is not binding. Elapsed duration is never expected to match.
- Current tests also reproduce the same result with one and four threads
  because work assignment and incumbent reduction are stable. The thread
  count is still part of the run configuration and should be recorded.
- `--seed` changes deterministic construction/neighborhood choices. It is not
  a source of nondeterminism and has no security meaning.
- A binding wall-clock deadline can stop at different candidate boundaries on
  different machines or under different load. Use a comfortably non-binding
  deadline when exact replay matters.
- Reversing both input arrays is tested to preserve the full objective value on
  the current and 8/77 fixtures. It does not promise byte-identical placement
  layouts because stable IDs follow input order.

## Current quality and performance evidence

On the 6-container/57-item fixture, the fast preset places 53 items and packs
587,815.524 volume, improving the old saved result of 49 items and 582,885.612
volume. On the generated 8-container/77-item fixture, fixed portfolio tests
place 73 items and pack 694,614.920 volume.

Native ARM64 Criterion measurements include independent final validation.
Fast/balanced/thorough means are approximately 0.839/2.451/3.860 ms on 6×57
and 1.048/3.295/5.490 ms on 8×77. These measurements are regression evidence,
not cross-machine latency guarantees.

## Known limits

- Orientation restrictions, weight, fragility, stacking, and item priorities
  are not part of the compatibility input.
- The report loads Three.js from external CDNs.
- Coverage-guided fuzzing requires the isolated nightly package documented in
  `../fuzz/README.md`.
- The checked-in dual-platform workflow still needs its first hosted run before
  current post-integration ARM64 macOS and x86-64 Linux evidence is complete.
