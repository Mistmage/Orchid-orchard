# Lilac Extension Development

## Prerequisites

1. Build the host: `cargo build -p lilac-host`
2. Load unpacked extension (Chrome: `chrome://extensions` → Developer mode → Load unpacked → `extension/`).
   Copy the generated extension ID.
3. Register host: `powershell -File scripts\install-host.ps1 -ExtensionId <id>`
   (Linux/macOS: `scripts/install-host.sh /path/to/lilac-host <ext_id>`)
4. Restart browser.

## Stable extension ID

Chrome derives the ID from the unpacked path unless a `key` is present in
`manifest.json`. For a stable ID across machines, generate a keypair and add
`"key": "..."` to the manifest, then re-run install with that ID.

Firefox uses the fixed id `lilac@orchid` (`browser_specific_settings.gecko.id`).

## Architecture

- `background.js`: service worker, single native port with reconnect + queue.
  Stateless: all data in the records root.
- `popup/`: quick-capture UI (save tab, save selected, recent, collections,
  tags, search, edit, open/change folder).
- Full-page manager view is a future item (see `Lilac plan/Phases 6+ ideas.md`).

## Adding a new host method

1. Add a branch in `dispatch` in `crates/lilac-host/src/main.rs`.
2. Add a message type in `background.js`.
3. Wire into popup if needed.
4. Update `docs/native-host-protocol.md`.
