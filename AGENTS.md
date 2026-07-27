# BoxPacker Development Instructions

These instructions apply to the entire project.

1. Treat `../oldBoxPackerForDeletion` as read-only reference material. Preserve
   its working-tree state and do not port its packing or scoring implementation.
2. Keep compatibility I/O, exact geometry, solver backends, independent
   validation, and reporting separated at their documented boundaries.
3. Prefer `devenv test` for the full project check. If host policy prevents it,
   run the closest pinned-toolchain checks available and report the limitation.
4. Keep `README.md` focused on new users: what the application does, a
   copyable first run, the input/output contract, and common options. Put
   detailed solver and reproducibility semantics in `docs/usage.md`, historical
   backend evidence in `docs/bakeoff/M2.5.md`, and architectural decisions in
   `docs/decisions/`.
5. Preserve the legacy JSON field names and CLI defaults unless a compatibility
   change is explicitly requested. Convert dimensions to exact scaled-integer
   geometry once at the input boundary, and independently validate every
   solver result before compatibility output or report rendering.
6. Treat `Cargo.toml` as the sole release-version source. `flake.nix`, the CLI
   `--version` output, release tags, and GitHub release assets must derive from
   or be checked against that package version. Preserve the draft-until-all-
   platforms-succeed behavior in `.github/workflows/release.yml`.
