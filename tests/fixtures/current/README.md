# Current compatibility fixture

These files are immutable compatibility snapshots copied from the tracked
files in `../../../../oldBoxPackerForDeletion`:

- `input.json` is the current 6-container, 57-item input.
- `saved_output.json` is the saved result with 49 placed and 8 unplaced items.

They were copied without running or importing the old packing and scoring
implementation. Their source SHA-256 digests are:

```text
0bd68c67409195ae2d70369fcef1ceefc9978f31a3a3a91c56f6dba0fea06b37  input.json
2f6c48ac2051350654fe870bf77d8a6690bbf016ad485345a72f3168c4a9e5f8  saved_output.json
```

The source input has trailing spaces on line 73. They are intentionally
retained, so an unfiltered `git diff --check` reports that fixture line.
