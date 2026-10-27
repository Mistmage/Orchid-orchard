# Phases 6+ — Ideas & Future Work

Captured while building v0.1. Not scoped for v0.1; parked here so nothing is lost.
Some items may be pulled into Phase 5 if cheap (see `Phase 5 - Forward-compat and later.md`).

## Phase 6 — UX polish & richer views

### Visual look
- Dark/light theme, accent color, density settings (compact/comfortable).
- Better typography for record cards; favicons from URL host.
- Empty-state illustrations; loading skeletons.
- Card vs. list view toggle; resizable popup.

### More comfortable controls
- Keyboard shortcuts: save tab, save selected, reopen, search focus, open folder.
- Command palette (Ctrl/Cmd+K) over records.
- Right-click context menu on pages: "Save to Lilac", "Snapshot", "Tag".
- Drag-and-drop tabs into popup to save; drag records between collections.
- Bulk actions: tag/delete/move multiple records.
- Undo for delete (soft-delete to a trash collection for N days).

### Full-page view (not just popup)
- A dedicated tab/page (`lilac.html`) for browsing the whole store: sidebar (Recent/Collections/Tags/Search), main list, detail pane.
- Opens in a browser tab via `chrome.tabs.create` on the extension's own page.
- Allows larger editing surface, multi-select, keyboard nav.
- Popup becomes a quick-capture shortcut; full page is the manager.

### Smarter filtering
- Filter by title substring, domain, tag, type, status, date range.
- Combine filters (AND/OR) with a small query builder.
- Saved filters / smart collections (a collection whose body is a query, not a list).
- Sort by title, created, modified, view count, domain.
- Domain grouping: "all records from github.com".

### Record naming (human-readable)
- Record filename/title derived from page title, with optional rules:
  - Affix/prefix/suffix based on domain (e.g. `github.com - <title>`).
  - Replace/strip rules (remove " - GitHub", site name noise).
  - Truncate with ellipsis; collision handling appends id.
  - User-defined naming templates (e.g. `{domain} — {title} ({date})`).
- Rule engine: per-domain or global, ordered, dry-run preview.
- Slug frozen at creation (id immutable) but display title editable.

## Auto-extractable page data (catalog for future)

Things Lilac could capture automatically from a page/tab without user input.
Each becomes optional metadata on a `page` record (all round-trip as open metadata):

Identity & location
- url, canonical url (from `<link rel="canonical">`), origin, host, path, query
- title, og:title, twitter:title
- favicon (from `<link rel="icon">` or /favicon.ico), theme color

Content & structure
- meta description, og:description, keywords
- author, publisher, site name (og:site_name)
- language (html lang, content-language)
- published/modified dates (article:published_time, etc.)
- word count, reading time estimate
- headings outline (h1/h2/h3)
- selected text / snippet (from user selection at save time)

Media & assets
- og:image / twitter:image (preview thumbnail)
- video/audio presence

Page state
- scroll position, form inputs (best-effort, privacy-sensitive)
- selected text at save time

Browser context
- referrer (source tab), opener chain
- window/tab position (hint only, per cross-browser rule)
- time of day, device

Derived / smart
- domain category (news, docs, video, …)
- duplicate detection (same canonical url)
- language detection

Privacy note: capture must be opt-in per field; sensitive fields (form inputs, selection) off by default. Store as open metadata so unknown extractors round-trip.

## Phase 7 — Sync & conflict UX
- Conflict detection UI (show conflicted files, side-by-side).
- Merge helpers for collection bodies.
- Per-record "changed outside Lilac" badge + one-click re-read.
- Sync status indicator (last sync time from file mtime).

## Phase 8 — Snapshot capture
- "Snapshot page" command (Chrome pageCapture → MHTML asset).
- Offline reading view; snapshot as `type: snapshot` or `snapshot:` ref on page.
- Asset content-addressed storage + dedup.
- Firefox equivalent (if available) or manual save.

## Phase 9 — Wisteria handoff
- Promote a Lilac record to Wisteria record (add authority/lineage/policy/state).
- Isotope lineage tracking.
- Shared record store between Lilac and Wisteria.

## Phase 10 — Platform & distribution
- Chrome Web Store / Firefox AMO packaging.
- Auto-update native host.
- Cross-platform installers (msi/dmg/deb).
- Telemetry: none (local-first principle). Optional anonymous usage if ever.

## Notes
- Phase 5 may pull in snapshot capture prototype if cheap; otherwise Phase 8.
- Keep v0.1 scope small ("and stop there"); phases 6+ are explicitly post-v0.1.
