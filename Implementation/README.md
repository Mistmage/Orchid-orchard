---
topic: Implementation
type: legend
status: active
---

# Implementation

Legend for implementation notes, tools, future work, tasks, and goals.

## Structure

- `Tools/` - tooling notes (install, usage, future).
- `Tools/Future/` - tools worth adopting later, not now.
- `Tasks/` - one note per task; status folders: `pending/`, `active/`,
  `deferred/`, `finished/`, `cancelled/`.
- `goals/` - goal notes by horizon: `Current/`, `Intermediary/`, `final/`,
  `reached/`; `goals/goal.md` organises them into milestones.
- `Process/Review/` - audits and process notes (what was claimed vs. what
  actually shipped).
- Topic folders (`Markdown/`, `Config/`, `Obsidian/`, ...) - notes about a
  subject, with state subfolders (`Fact/`, `Change/`, `Parity/`, ...).

## Convention

Every note in this folder uses YAML frontmatter with at least:
- `topic`: short topic name
- `type`: note | tool | future | legend | task | goal | goal-current | goal-intermediary | goal-final
- `status`: active | planned | deferred | finished | reached | cancelled

Related notes link via `related:` list of paths, relative to the note's own
folder.
