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