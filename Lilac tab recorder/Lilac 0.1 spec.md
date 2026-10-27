Lilac Specification v0.1
Purpose

Lilac is a local-first record storage system for browsers.

Instead of storing bookmarks, tab groups, sessions and related metadata inside browser databases, Lilac stores them as portable files.

Primary goals:

Human-readable
Git-friendly
Syncthing-friendly
Browser-independent
User-editable
Extensible
Future-compatible with Wisteria

Lilac treats tabs, bookmarks, sessions and other browser artifacts as records.

Core Principles
Files are the database

The filesystem is the source of truth.

Plain Text
Record
↓
Markdown + YAML

Indexes are caches.

Everything must be reconstructable from files alone.

Browsers are clients
Plain Text
Firefox
Chrome
Edge
Brave
Vivaldi
↓
Lilac

No browser is authoritative.

Metadata is open

Users may add arbitrary YAML properties.

Lilac only reserves a small namespace.

Everything else must survive round-trips.

Valid:

YAML
rating: 5
author: Mistere
foo: bar
 
my_company:
project: Orchid
priority: high
Links are first-class

All metadata fields may contain references.

Example:

YAML
project: [[Orchid]]
 
related:
- [[Rust]]
- [[Wisteria]]
 
parent: [[Research]]

Lilac does not distinguish strongly between:

Plain Text
Property
Reference
Link

Everything is data.

Record Format

A record consists of:

Plain Text
YAML Frontmatter
+
Optional Markdown Body

```
Lilac strongly prefers references over textual enumeration values. Whenever a value represents a concept rather than a literal measurement, a link to a record should be used.

Examples:

Preferred: status: [[Reading]]

Allowed: status: reading

Preferred: project: [[Orchid]]

Allowed: project: Orchid

Preferred: tags: [[Rust]]

Allowed: tags: [rust]

References enable metadata to become self-describing, queryable, extensible and interoperable with future Wisteria record systems.
```



Example:

Markdown
---
id: 89a13f2
type: page
 
title: SeaORM
 
url: https://sea-orm.org
 
created: 2026-10-26T12:00:00Z
modified: 2026-10-26T12:30:00Z
 
tags:
- rust
- orm
 
project: [[Orchid]]
---
 
Research candidate.
Reserved Fields

Reserved fields have defined semantics.

Identity
YAML
id:
type:
version:

Example:

YAML
id: rec_89a13f2
type: page
version: 1
 
Display
YAML
title:
icon:
color:
description:
Timestamps
YAML
created:
modified:
opened:
last_viewed:
References
YAML
parent:
related:
links:

Example:

YAML
parent: [[Rust]]
 
related:
- [[Database]]
- [[Research]]
Classification
YAML
tags:
categories:
status:

Example:

YAML
status: active
 
tags:
- rust
- web
Record Types

Record types are conventions.

Not hardcoded.

page

A web page.

YAML
type: page
url: https://sea-orm.org
collection

Equivalent to OneTab session.

YAML
type: collection

Body:

Markdown
- https://sea-orm.org
- https://diesel.rs
- https://sqlx.rs
bookmark

Permanent saved page.

YAML
type: bookmark
snapshot

Archived content.

YAML
type: snapshot
note

User note.

YAML
type: note
snippet

Small extracted content.

YAML
type: snippet
Browser Metadata

Unlike normal record data, browser metadata is optional and ephemeral.

Open State
YAML
open:

Example:

YAML
open: firefox

meaning:

Plain Text
Currently open in Firefox
Position
YAML
position:

Example:

YAML
position:
browser: firefox
window: 2
tab: 44

This information may become invalid.

Lilac must treat it as hints.

Not truth.

Views

Future Wisteria concept.

YAML
views:
- firefox
- chrome

or

YAML
views:
- workspace: research
Record Relationships

Records form a graph.

Not merely a folder structure.

Parent
YAML
parent: [[Research]]
Related
YAML
related:
- [[Rust]]
- [[Databases]]
Derived
YAML
derived_from: [[SeaORM]]
Forks

Future Wisteria compatibility.

YAML
forked_from: [[YouTube]]

Equivalent to isotope lineage.

Collection Format

A collection can reference records.

Instead of URLs alone.

Example:

Markdown
---
type: collection
title: ORM Research
---
 
- [[SeaORM]]
- [[Diesel]]
- [[SQLx]]

or mixed:

Markdown
- [[SeaORM]]
- https://diesel.rs
Queries

Queries may appear as metadata.

Example:

YAML
query: tag:rust

or

YAML
query: |
type=page
status=active

Query syntax remains implementation-defined.

Index

Lilac may generate:

Plain Text
index.json

containing:

JSON
{
"id":"89a13f2",
"title":"SeaORM",
"tags":["rust"]
}

This file is disposable.

Deleting it must never destroy information.

Native Helper API

Minimal API:

Plain Text
save_record
update_record
delete_record
 
load_record
 
search_records
 
rebuild_index

Nothing else initially.

Synchronization

Lilac itself does not sync.

Supported sync:

Plain Text
Git
Syncthing
Dropbox
OneDrive
Nextcloud
Rsync

because records are ordinary files.

Conflict Philosophy

A conflicted file is preferable to a hidden merge.

Users must be able to inspect conflicts with ordinary tools.

This aligns with:

Plain Text
File-first
Human-readable
Git-friendly

design goals.

Wisteria Compatibility

Lilac records are intentionally more general than tabs.

Every Lilac record should be promotable into a future Wisteria record.

Future additions:

YAML
authority:
YAML
authority: client
authority: server
YAML
lineage:
YAML
lineage: rec_123
YAML
policy:
YAML
policy:
singleton: true
auto_snapshot: true
YAML
state:
YAML
state:
live: false
evicted: true
Final Definition

Lilac is a local-first, Markdown/YAML based record store for browser data.

Tabs, bookmarks, sessions, notes, snapshots and future Wisteria records are all represented as extensible records stored as ordinary files. Browser-specific state may be attached as metadata, but the canonical representation remains browser-independent and filesystem-native. The filesystem is the database, indexes are caches, links are first-class, and arbitrary user-defined metadata is preserved without schema restrictions.