---
topic: exact-folder-placement
type: task
status: active
related:
  - crates/lilac-core/src/storage.rs
  - extension/popup/popup.js
---

# T1 — Exact folder placement + subtrees

Basic folder placement is done: `folder` field on records, `record_path` honors
it (sanitized, no traversal), rules' `move` action sets it, UI exposes folder
field + badge + settings.

Remaining: nested subtree refinement, UI for moving between folders, bulk move.