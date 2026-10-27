# Phase 5 — Forward-Compat & Later

**Goal:** Harden Lilac for real-world sync/conflict use and lay the additive groundwork for Wisteria, without changing the v0.1 core. This phase is about **durability and future-proofing**, not new user features.

**Milestone:** post-M3. **Depends on:** Phases 1–4.

## Scope

In scope:
- Reserve + document Wisteria fields (`authority`, `lineage`, `policy`, `state`, `forked_from`) — present in schema, no-op semantics in v0.1.
- Snapshot type preparation (Q2 = A-aware-of-B): `snapshot`/`snapshot_format` asset refs, assets folder, content-addressed storage, additive population path.
- Sync/conflict hardening: conflicted-file handling, mtime-based change detection, rebuild robustness.
- Cross-platform native host packaging (installers for Win/Linux/macOS, Chrome + Firefox).
- Documentation: record format spec, native host protocol, extension dev guide.
- (Optional) snapshot capture prototype behind a flag.

Out of scope: Wisteria browser, isotope/policy engine, custom renderer, history replacement.

## Wisteria field reservation (no-op v0.1)

Fields added to schema and round-tripped, but no behavior attached in v0.1:

- `authority: client | server` (default client for notes, server for pages later).
- `lineage: <record id>` — origin record.
- `forked_from: <record id>` — isotope lineage.
- `policy: { singleton, auto_snapshot, ... }` — reserved.
- `state: { live, evicted, ... }` — reserved.

These are written only if present in input; never synthesized by v0.1. Purpose: a Wisteria record can be read by Lilac without loss, and a Lilac record can be promoted later without migration.

## Snapshot-awareness (Q2 design, additive)

Page records may optionally carry:
```yaml
snapshot: sha256:<hash>          # or relative path under Assets/
snapshot_format: mhtml            # mhtml | singlefile | pdf | ...
snapshot_created: 2026-...
```

Design rules so adding MHTML later is **additive, not a rewrite**:
1. **Metadata is primary.** A `page` record always has `url`/`title`/tags/body; snapshot is an attachment, never the identity.
2. **Assets are separate.** Snapshot bytes live under `<records root>/Assets/`, referenced by hash or relative path. Records never embed binary.
3. **Content-addressed preferred.** Asset filename = `<hash>.<ext>`; dedup is free; integrity checkable.
4. **Population is a new action.** "Snapshot this page" = fetch MHTML → write asset → `update_record` adding `snapshot` field. No change to save flow, schema, or existing records (field simply absent).
5. **Type remains `page`.** No new record type required for a snapshotted page; `type: snapshot` stays for standalone archived content. (Could optionally introduce `type: snapshot` as a first-class record later — also additive.)
6. **Reopen ignores snapshot.** Reopen uses `url`; snapshot is for offline reading (future Wisteria feature). v0.1 doesn't render snapshots.

Cost: slightly slower to add (asset plumbing + hashing), but avoids a schema migration and keeps v0.1 records valid forever.

## Sync & conflict hardening

- **Conflict philosophy (from spec):** a conflicted file is preferable to a hidden merge. Lilac must never silently merge.
- On read, if a file has conflict markers (`<<<<<<<`), surface it as a record with a `conflict: true` flag (or skip + warn) rather than crashing.
- Rebuild index must tolerate unreadable/conflicted files (skip + log), not fail entirely.
- Mtime-based staleness already in Phase 4; document.
- Recommend users put records root under git/syncthing; provide a `.gitignore` template (ignore `.lilac/` config root entirely; track `Records/` + `Assets/`).

## Packaging

- Build `lilac-host` for Windows (`.exe`), Linux, macOS (universal or per-arch).
- Installers: PowerShell (Windows), shell (Linux/macOS) that write native messaging manifests + (Windows) registry.
- Uninstall reverses everything.
- Extension published as unpacked (dev) + store packages later (Chrome Web Store / Firefox AMO) — out of scope for v0.1 dev, but manifest must be store-ready (no dev-only permissions).

## Documentation deliverables

- `docs/record-format.md` — canonical on-disk format, reserved fields, reference syntax.
- `docs/native-host-protocol.md` — message envelope, methods, errors.
- `docs/extension-dev.md` — MV3 setup, native host registration, dev workflow (stable extension ID via manifest key).
- `docs/sync.md` — recommended sync setups, conflict handling, `.gitignore`.

## Optional: snapshot prototype (flag-gated)

Behind a config flag `snapshots.enabled = false` by default:
- "Snapshot page" command fetches MHTML via `chrome.pageCapture.saveAsMHTML` (Chrome) or equivalent; writes asset; updates record.
- Validates the additive design end-to-end.
- Not required for v0.1 release; validates Phase 5's claims.

## Acceptance criteria (Phase 5 done when)

- Wisteria fields round-trip (present in input → preserved; absent → not synthesized).
- A snapshotted page (manually crafted record with `snapshot:` field) loads without error; asset path resolves.
- Conflicted file doesn't crash rebuild; index skips it with a log entry.
- Install/uninstall works on Windows (and ideally one Unix) for Chrome + Firefox.
- Documentation covers format, protocol, dev, sync.
- `.gitignore` template provided.

## Risks

- Chrome `pageCapture` availability and MHTML fidelity vary; prototype may be deferred.
- Cross-platform packaging effort may dominate; scope to Windows-first if needed, document others.
- Over-engineering risk: keep Phase 5 additive; resist feature creep into Wisteria.

## Deliverables

- Hardened, documented, packaged v0.1.
- Clear handoff note: "what Wisteria needs from Lilac's record model" (see `Wisteria draft.md`).

## Handoff to Wisteria

Lilac v0.1 delivers the seed: a portable, file-based, extensible record store with types, references, open metadata, and Wisteria-compat fields. Wisteria's job (far future) is to make the browser itself record-native — tabs as records, isotopes, policy engine, record-native views. Lilac's records are the substrate; nothing in v0.1 should need to change for that future except additive fields and new types.
