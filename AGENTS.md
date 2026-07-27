# BoxPacker Development Instructions

These instructions apply to the entire project.

1. Treat `../oldBoxPackerForDeletion` as read-only reference material. Preserve
   its working-tree state and do not port its packing or scoring implementation.
2. Keep compatibility I/O, exact geometry, solver backends, independent
   validation, and reporting separated at their documented boundaries.
3. Prefer `devenv test` for the full project check. If host policy prevents it,
   run the closest pinned-toolchain checks available and report the limitation.
