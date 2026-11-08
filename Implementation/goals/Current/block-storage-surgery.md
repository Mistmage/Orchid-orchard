---
topic: goal-current-block-storage-surgery
type: goal-current
status: active
related:
  - ../goal.md
  - ../../Tasks/active/T1-exact-folder-placement.md
---

# Current goal — Block storage surgery

Wire `splice_block` + WAL + `BlockEditor` into record writes (surgical edits,
external-edit reconciliation). Unlocks Orchid note editing. Foundation in place
(`apply_block_edit`, `BlockEditor`, `Wal`, `reconcile_text`).

Shipped: `RecordSession` in `crates/lilac-core/src/session.rs` with path-based
edits, WAL replay/orphan handling, external reconciliation, stale-block refusal
and undo; Tauri commands and the Solid editor are wired to it. See
`../../Markdown/Design/record-session.md`.

Remaining: redo, multi-window coordination, frontmatter/metadata editing, and
richer per-block-type leaf components.