# Phase 2 — Native Host + Save Current Tab

**Goal:** Wire Phase 1's Rust core to the browser via a native messaging host, and ship the first end-to-end flow: **save current tab → a record file on disk**.

**Milestone:** M1. **Depends on:** Phase 1.

## Scope

In scope:
- Rust native messaging host binary (stdin/stdout, length-prefixed JSON).
- Minimal API exposed over native messaging: `save_record`, `update_record`, `delete_record`, `load_record`, `search_records`, `rebuild_index`.
- MV3 extension skeleton: manifest, service worker, popup, `connectNative`.
- "Save current tab" command: read tab url/title → build `type: page` record → save.
- Per-OS host registration (install/uninstall).
- Error handling and logging to `.lilac/logs/`.

Out of scope: collections, reopen, search UI, metadata editing.

## Native messaging protocol

Chrome/Chromium/Firefox native messaging: host reads 4-byte little-endian length prefix + JSON from stdin; writes same to stdout. Messages are single JSON objects.

Envelope:
```json
{ "id": "req-1", "method": "save_record", "params": { ... } }
```
Response:
```json
{ "id": "req-1", "ok": true, "result": { ... } }
```
or
```json
{ "id": "req-1", "ok": false, "error": { "code": "NOT_FOUND", "message": "..." } }
```

- `id` correlates request/response; host must echo it.
- Host must flush stdout after each response.
- Host must not write anything else to stdout (logs go to file only).
- On EOF on stdin → exit cleanly.

## Minimal API (over native messaging)

| Method | Params | Result |
|--------|--------|--------|
| `save_record` | `{ record: {...} }` (or `{ type, fields }`) | `{ id, path }` |
| `update_record` | `{ id, patch }` | `{ id, path }` |
| `delete_record` | `{ id }` | `{}` |
| `load_record` | `{ id }` | `{ record }` |
| `search_records` | `{ query }` | `{ results: [...] }` |
| `rebuild_index` | `{}` | `{ count }` |
| `get_config` | `{}` | `{ records_root, config_root }` |
| `set_records_root` | `{ path }` | `{}` |

Config root and records root come from `.lilac/config.yaml`; `set_records_root` updates it (with validation that path exists/writable).

## Rust host

- Binary name: `lilac-host` (or `lilac-host.exe` on Windows).
- On startup: load config; if missing, create default `.lilac/` and default records root (`~/Lilac`), write config.
- Main loop: read message → dispatch → respond. Handle parse errors without exiting.
- Logging: append to `.lilac/logs/host.log` with timestamps; never stdout.
- Graceful shutdown on stdin EOF.

## MV3 extension skeleton

Files:
```
extension/
├── manifest.json          # MV3
├── background.js          # service worker: native port management
├── popup/
│   ├── popup.html
│   ├── popup.js
│   └── popup.css
└── icons/
```

Manifest (key parts):
- `manifest_version: 3`
- `permissions: ["tabs", "storage", "nativeMessaging"]`
- `host_permissions: ["<all_urls>"]` (needed to read tab url/title reliably; revisit for least-privilege later)
- `action.default_popup: popup/popup.html`
- `background.service_worker: background.js`

Service worker responsibilities:
- Maintain a single native port via `chrome.runtime.connectNative("lilac_host")` (or `browser.runtime.connectNative`).
- Reconnect on disconnect (MV3 worker restart).
- Queue messages while disconnected; flush on connect.
- Expose `send(method, params) -> Promise`.

## "Save current tab" flow

1. User clicks extension action (or keyboard shortcut).
2. Popup/service worker calls `chrome.tabs.query({ active: true, currentWindow: true })`.
3. Build record payload:
   ```json
   { "type": "page", "title": <tab.title>, "url": <tab.url>, "tags": [], "created": <now>, "modified": <now> }
   ```
   (No `id`; host assigns.)
4. `send("save_record", { record })`.
5. Host: assign `id` (`rec_<short>`), slugify title, write file under `Records/page/`, rebuild index entry, respond `{ id, path }`.
6. UI shows confirmation (toast in popup) with the record title.

Notes:
- `id` generation: short random hex (e.g., 7 chars) or counter; must be unique within records root. Check collision on save.
- Slug frozen at creation from title; if title missing, use host from url.
- No snapshot captured (Q2 = A).

## Per-OS host registration

Chrome native messaging host registration:
- **Windows:** registry key `HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts\lilac_host` → path to manifest JSON. (Or `HKEY_LOCAL_MACHINE` for all users — prefer CU for per-user.)
- **Linux:** `~/.config/google-chrome/NativeMessagingHosts/lilac_host.json`.
- **macOS:** `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/lilac_host.json`.

Firefox: similar paths under `Mozilla/NativeMessagingHosts` and Firefox profile dirs; manifest includes `allowed_extensions`.

Manifest JSON (host side):
```json
{
  "name": "lilac_host",
  "description": "Lilac record storage host",
  "path": "/abs/path/to/lilac-host[.exe]",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://<EXT_ID>/"],
  "allowed_extensions": ["lilac@orchid"]
}
```

Provide install/uninstall scripts (PowerShell + bash) that write the manifest and registry/files. Document manual steps too.

## Error handling

- Native host not installed → extension shows "host not found" with link to install instructions; no crash.
- Records root unwritable → host returns error with path; extension surfaces it.
- All errors logged to `.lilac/logs/host.log` with request id.

## Acceptance criteria (M1)

- Install host + load extension in Chrome (and Firefox).
- Click "Save tab" on a page → a file `Records/page/<slug>--<id>.md` exists with correct frontmatter + body (empty).
- File is readable in any text editor / Obsidian.
- Re-running on same tab creates a second record (no dedup in v0.1 — acceptable; dedup is later).
- Uninstall host → extension degrades gracefully with clear message.
- `.lilac/index.json` reflects the new record after save.

## Tasks

- [ ] Scaffold Rust host binary with config load + main loop.
- [ ] Implement native messaging read/write (length-prefixed JSON).
- [ ] Expose minimal API, dispatch to Phase 1 core.
- [ ] Assign ids, slugify titles.
- [ ] Scaffold MV3 extension (manifest, service worker, popup).
- [ ] Implement connectNative + reconnect + message queue.
- [ ] Implement "Save current tab" end-to-end.
- [ ] Install/uninstall scripts for Chrome + Firefox on Win/Linux/macOS.
- [ ] Error UX in popup + logging.

## Risks

- MV3 service worker lifecycle: native port drops when worker sleeps; must reconnect lazily. Mitigate with a `ensurePort()` before every call.
- Extension ID changes during development (unpacked) → native host `allowed_origins` must be updated; document dev workflow (use a fixed key in manifest for stable ID).
- Windows path/registry quoting in install scripts.
- Firefox native messaging requires extension ID in `allowed_extensions`; test both browsers.

## Deliverables

- `lilac-host` binary for at least Windows (and ideally Linux/macOS).
- MV3 extension that saves the current tab end-to-end.
- Install/uninstall scripts + docs.
- M1 demo: save a tab, open the `.md`, see your data.
