---
topic: extension-save-pipeline
type: task
status: active
related:
  - ../../../extension/background.js
  - ../../../extension/popup/popup.js
  - ../../../crates/lilac-host/src/lib.rs
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
---

# T9 — Save preview, exclusion, instant save

Shipped:

- The save pipeline is one plan and one commit in Rust:
  `crates/lilac-core/src/save_plan.rs` builds `SavePlan { source, records,
  duplicates, warnings }` without writing, and `commit_plan` writes exactly the
  drafts it is given. The host exposes `plan_save`, `commit_save` and `plan_path`
  (recomputes the shown path after an edit).
- Plans run in the same order as a save: naming, folder placement (rejecting
  escapes with a warning), rules, duplicate detection and title fallback.
  Duplicate pages are reported with `duplicate_of` and in `duplicates`; they are
  not dropped yet (T10 owns that decision).
- The popup has a preview view. One record opens in detail with every field,
  the body, the file path and its warnings; several records show a list with an
  include checkbox per row and a Details button. Both forms allow corrections to
  title, folder, tags, status, description, URL, simple extra fields and body.
  Cancel writes nothing; commit reports written, skipped and failed records.
- Excluding a page also removes its `[[id]]` entry from the set it belongs to,
  and a corrected page title/URL is copied into that set's entries, so a saved
  collection cannot point at an old value or a record that was not written.
- The header saves (tab, selection, group, window, snapshot) and a new
  Workspaces save all go through `startSave` → preview. Workspace update stays a
  direct in-place update. A snapshot captures its asset before the plan so the
  preview can show it; cancelling leaves only an unused content asset.
- **Save page instantly** replaces the old one-off confirm. It is off by
  default, persists in extension storage, and skips the preview only for a
  single page save. Rules, naming, placement and duplicate detection still run.

Verified: core plan tests cover preview/commit agreement, exclusion, duplicate
reporting, folder warnings, workspace geometry, edited folder/title paths and
set pruning. Host tests cover preview/commit agreement, exclusion, duplicates,
broken rules and paths.

## Goal

Every save is inspectable before it becomes a file, and "write immediately" is a
deliberate setting rather than the only behaviour. Preview is not a second
implementation of saving: it is the dry run of the same plan that the commit
uses.

## Requirements

- Preview any save: one page, selected tabs, a tab group, a window, a snapshot
  and a collection.
- One record: show it in detail — type, subtype, title, folder, filename, URL,
  tags, every metadata field, body, and the rules/warnings that produced it.
- Several records: show a larger list where one row can be opened in detail and
  individual records can be excluded. Exclusion survives until commit; an
  excluded record is never written.
- Fields can be corrected in the preview before commit, and the correction is
  what gets written (including the folder and the filename).
- Warnings (failed rule, duplicate, missing required field) are visible next to
  the record they belong to, not in a log.
- Cancel writes nothing. A commit reports per-record success and failure instead
  of stopping at the first error.
- A setting, **Save record instantly**, skips the preview for page saves. It is
  off by default, and rules, de-duplication and post-processing still run.
- A plan is data: `{ records: [ draft ], warnings, duplicates, source }`. The
  preview returns it and the commit accepts it, so the two cannot drift.

## Design notes

The plan must be produced by the same core path as a normal save, after
classification (T11), de-duplication (T10) and placement (T1). The host exposes
`plan_save` and `commit_save`; the extension renders the plan. This keeps the
extension thin and means Orchard can later show the same preview for imports.

## Done when

- Preview output and written files agree exactly in a host test (same fields,
  folder, filename).
- Excluded drafts are absent from disk and reported in the result.
- The instant-save setting is persisted and honoured.
- A preview with a broken rule still saves the record and reports the warning.

## Open questions

- Should preview be the default for collections only, or for every save? Current
  intent: every save, with instant save as the opt-out.
