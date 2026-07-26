# BoxPacker Rewrite Agent Instructions

These instructions apply to the entire rewrite project.

1. Read `REWRITE_PLAN.md` completely before making changes. It is the canonical
   specification, task tracker, decision register, and handoff record.
2. Treat `../oldBoxPackerForDeletion` as read-only reference material. Preserve
   its working-tree state and do not port its packing or scoring implementation.
3. Work from the first unchecked task in the current milestone unless the
   handoff snapshot explicitly identifies a different next task or blocker.
4. Keep compatibility I/O, exact geometry, solver backends, independent
   validation, and reporting separated at their documented boundaries.
5. Run the checks required by the active milestone. Prefer `devenv test`; record
   exact host-policy limitations and fallback checks when devenv cannot run.
6. Before ending a handoff, update the plan's dated snapshot, task statuses,
   decisions, verification evidence, blockers, changed files, and exact next
   task.

