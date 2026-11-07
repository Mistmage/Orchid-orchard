# Lilac Native Host Protocol

Chrome/Firefox native messaging: host reads 4-byte little-endian length + JSON
from stdin; writes 4-byte LE length + JSON to stdout.

## Host executable

The host is not a separate binary. The native messaging manifest points at
`orchard.exe`; `orchard.exe` detects host mode (Chrome/Firefox pass
`--parent-window` / `-parent-window`, and the browser gives the process stdio
pipes) and runs the same message loop. `orchard.exe --host` forces host mode
for manual testing.

## Envelope

Request:
```json
{ "id": "req-1", "method": "save_record", "params": { ... } }
```
Response:
```json
{ "id": "req-1", "ok": true, "result": { ... } }
```
or
```json
{ "id": "req-1", "ok": false, "error": { "code": "ERROR", "message": "..." } }
```

`id` correlates request/response. Host must echo it. Logs go to file only
(`~/.lilac/logs/host.log`), never stdout.

## Methods

| Method | Params | Result |
|--------|--------|--------|
| `save_record` | `{ record, body? }` | `{ id, path }` |
| `update_record` | `{ id, patch?, body? }` | `{ id, path }` |
| `delete_record` | `{ id }` | `{}` |
| `load_record` | `{ id }` | `{ record, body, path }` |
| `search_records` | `{ query }` | `{ results: [IndexEntry] }` |
| `rebuild_index` | `{}` | `{ count }` |
| `get_config` | `{}` | `{ records_root, config_root }` |
| `set_records_root` | `{ path }` | `{ records_root }` |
| `open_folder` | `{ path? }` ("records"/"config"/abs path) | `{ path }` |
| `move_record` | `{ id, folder }` | `{ id, path }` |
| `move_file` | `{ src, dst }` | `{ src, dst }` |

## Installation

- Windows: registry `HKCU\Software\Google\Chrome\NativeMessagingHosts\lilac_host`
  and `HKCU\Software\Mozilla\NativeMessagingHosts\lilac_host` → manifest JSON path.
- Linux/macOS: manifest in `~/.config/google-chrome/NativeMessagingHosts/` and
  `~/.mozilla/native-messaging-hosts/`.

Manifest declares `allowed_origins` (Chrome) and `allowed_extensions` (Firefox).
See `scripts/install-host.ps1` / `install-host.sh`.
