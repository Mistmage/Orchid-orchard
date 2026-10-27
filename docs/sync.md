# Sync & Conflict Handling

Lilac does not sync itself. Records are ordinary files, so any file sync works:
Git, Syncthing, Dropbox, OneDrive, Nextcloud, rsync.

## Recommended setup

- Put the records root in a synced folder (e.g. a Git repo or Syncthing folder).
- Keep the config root (`~/.lilac/`) out of sync (it is machine-specific and
  regenerable). The repo's `.gitignore` ignores `Lilac/` (dev records) and the
  user should gitignore their records root's parent's `.lilac/`.

## Conflict philosophy

A conflicted file is preferable to a hidden merge. Lilac never silently merges.
- Rebuild tolerates unreadable/conflicted files (skips + continues).
- Conflict markers (`<<<<<<<`) make a file unparseable → skipped in index.
- Users inspect conflicts with ordinary tools (git, diff).

## .gitignore template for a records root

```
# ignore machine-specific config
.lilac/
.orchard/

# ignore OS junk
.DS_Store
Thumbs.db
```

Track `Records/` and `Assets/`.
