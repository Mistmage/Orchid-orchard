Lilac Specification Addendum A
Metadata, Properties, References, and Semantic Architecture
1. Philosophy

Traditional metadata systems treat a record as data and metadata as annotations.

YAML
title: SeaORM
status: reading
project: orchid

Lilac instead treats both records and metadata as part of the same graph.

YAML
status: [[Reading]]
project: [[Orchid]]

The distinction between:

record
metadata
schema
tag
type

is deliberately reduced.

Everything can potentially be represented as a record.

2. Semantic Model

Every record is a node.

Every property is a relation.

Every linked value is a node.

Example:

YAML
status: [[Reading]]
project: [[Orchid]]

Graph:

Plain Text
Current Record
│
├─ status ─────► Reading
│
└─ project ────► Orchid
 
3. Property Types

A property may point to:

YAML
status: [[Reading]]

Reference Value

YAML
rating: 80

Literal Value

YAML
started: 2026-10-26

Temporal Value

YAML
tags:
- [[Rust]]
- [[ORM]]

Reference Collection

YAML
data:
score: 50
views: 1000

Structured Object

4. Property Records

Properties are first-class records.

Example:

Markdown
---
type: property
 
name: rating
 
value_type: number
 
min: 0
max: 100
 
display: percentage
---
 

Meaning:

YAML
rating: 6

is interpreted as:

Plain Text
6 / 100

because the property definition specifies that scale.

Another property:

Markdown
---
type: property
 
name: weight
 
value_type: number
 
unit: kg
---

Then:

YAML
weight: 80

means:

Plain Text
80 kg
5. Preferred Representation

Lilac distinguishes between semantic concepts and raw values.

Semantic concepts should use references.

Preferred:

YAML
status: [[Reading]]
project: [[Orchid]]
author: [[Mistere]]

Allowed:

YAML
status: reading
project: Orchid
author: Mistere

The link form is preferred whenever the value represents an entity or concept.

6. Property Identity

Internally:

YAML
status:

is equivalent to:

YAML
[[status]]:

Conceptually.

Meaning:

YAML
status: [[Reading]]

can be interpreted as:

Plain Text
[[status]] → [[Reading]]

even if users omit brackets around property names.

7. Explicit Property References

Advanced users may write:

YAML
[[status]]: [[Reading]]
 

or

YAML
[[rating]]: 6

when explicit graph notation is desired.

Lilac should treat both forms as equivalent:

YAML
status: [[Reading]]

and

YAML
[[status]]: [[Reading]]
8. Value Categories

Lilac recognizes three broad categories of values.

Literals

Have intrinsic meaning.

YAML
rating: 80
created: 2026-10-26
count: 42
title: SeaORM
References

Meaning comes from another record.

YAML
status: [[Reading]]
project: [[Orchid]]
Structured Values

Meaning comes from internal structure.

YAML
dimensions:
width: 100
height: 50
9. Reference Resolution

Reference:

YAML
status: [[Reading]]

Resolution order:

Plain Text
Reading.md
 
Reading.record
 
Reading.yaml
 
Reading/

Implementation-defined.

Only requirement:

Plain Text
Reference
↓
Stable Record
10. Property Metadata

Property records may define:

Type
YAML
value_type: number
Range
YAML
min: 0
max: 100
Units
YAML
unit: kg
Cardinality
YAML
multiple: true

Example:

YAML
tags:
- [[Rust]]
- [[ORM]]
Validation
YAML
allowed:
- [[Reading]]
- [[Finished]]
- [[Dropped]]
Display
YAML
icon: book
color: blue
Query Behaviour
YAML
indexed: true
searchable: true
11. Taxonomy Records

Tags are not special.

Instead:

YAML
tags:
- [[Rust]]
- [[ORM]]

means:

Plain Text
Current Record
│
├─ tags ─► Rust
│
└─ tags ─► ORM

Rust and ORM are ordinary records.

12. Status Records

Example:

Markdown
---
type: status
 
icon: book
 
color: blue
 
transition:
- [[Finished]]
- [[Dropped]]
---

Used by:

YAML
status: [[Reading]]
13. Type Records

Instead of:

YAML
type: page

Lilac may prefer:

YAML
type: [[Page]]

where:

Markdown
---
type: record_type
---

Page itself defines expected behaviour.

14. Queryable Schema

Because properties are records, schema becomes queryable.

Example:

Plain Text
Show all properties

returns:

Plain Text
Status
Rating
Project
Authority
Parent
Related

without requiring hardcoded definitions.

15. User Extensions

Users may define entirely custom properties:

YAML
mood: [[Inspired]]
 
difficulty: 80
 
world:
region: [[North]]

No registration required.

Property records become optional enhancements rather than requirements.

16. Wisteria Compatibility

This architecture intentionally moves toward:

Plain Text
Everything is a Record

Including:

Plain Text
Page
Collection
Status
Reading
Property
Tag
Project
Authority
Workspace

The distinction between schema and content gradually disappears.

The graph itself becomes the schema.

Recommended Lilac Convention

For maximum compatibility and readability:

YAML
type: [[Page]]
 
status: [[Reading]]
 
project: [[Orchid]]
 
tags:
- [[Rust]]
- [[ORM]]
 
rating: 80

Use references whenever a value represents a concept.

Use literals whenever a value represents a measurement.

Treat property names as implicit records, and linked values as explicit records.

This keeps the files valid YAML, human-editable, Obsidian-friendly, Git-friendly, and already very close to the record graph model that Wisteria ultimately wants.