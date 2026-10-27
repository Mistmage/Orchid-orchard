# Lilac Extension (MV3)

Thin client for the Lilac record store. Talks to the Rust native host `lilac-host` via native messaging.

## Try it end-to-end (correct order)

1. Build the host:
   ```
   cargo build -p lilac-host
   ```
2. Load the extension first (so you get its ID):
   - Chrome: open `chrome://extensions`, enable Developer mode, click "Load unpacked", select this `extension/` folder. Copy the generated extension ID.
   - Firefox: open `about:debugging` → "This Firefox" → "Load Temporary Add-on" → select `extension/manifest.json`.
3. Register the native host with that ID:
   - Windows: `powershell -File ..\scripts\install-host.ps1 -ExtensionId <id from step 2>`
     - Records root defaults to `<current dir>\Lilac` (run from the project folder for dev). Override with `-RecordsRoot "D:\Notes\Lilac"`.
   - Linux/macOS: `../scripts/install-host.sh /path/to/lilac-host <ext_id> [records_root]`
4. Restart the browser.

## Records folder

- Default (no config): `~/Documents/Lilac`.
- Change anytime via the extension's "Change folder" button, or `set_records_root` API.
- "Open folder" button reveals the current location in the file manager.
5. Click the Lilac toolbar icon → "Save tab". Use "Open folder" to reveal where records were written (default `~/Lilac`).

## Dev notes

- Chrome extension ID is derived from the unpacked path unless a `key` is added to manifest.json. For a stable ID across machines, generate a keypair (Chrome docs: "Private key for signing") and add `"key": "..."` to manifest.json, then re-run install with that ID.
- Firefox uses the fixed id `lilac@orchid` (set in `browser_specific_settings.gecko.id`).
- The extension is stateless: all data lives in the records root (default `~/Lilac`), config in `~/.lilac`.

## Current capabilities (Phase 2)

- Save current tab → `Records/page/<slug>--<id>.md`
- Save selected tabs → collection record + per-tab page records
- List recent records; reopen a record or collection

Search/edit/tags UI arrive in Phase 4.