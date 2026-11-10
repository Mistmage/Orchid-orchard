---
topic: templates
type: design
status: active
related:
  - ../../../crates/lilac-core/src/template.rs
  - ../../../crates/lilac-core/src/record.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
  - ../../../apps/orchard/src/App.tsx
  - ../../../docs/orchard-dev.md
---

# Templates: declared masks as records

## A template is a record

Nothing new is introduced to the file format. A template is an ordinary record
whose `type` is `template` and whose `target_type` names the type it produces.
That means templates are visible to the same index, search, move, metadata and
round-trip code as every other record. A damaged template is a damaged file, not
a hidden entry in an app database.

`target_type` is required. `create_from_template()` refuses a template that
leaves it empty instead of guessing a type from the title or filename. The type
system stays declared: a template that cannot say what it produces does not
silently produce something else.

## Applying a template

1. Find the template by id and read it.
2. Check that it is a template and that `target_type` is non-empty.
3. Generate an id that no record under `Records/` uses.
4. Copy the record. Reset `id`, `type`, `target_type`, timestamps, `opened`,
   `last_viewed` and `folder`; set the type from `target_type`.
5. Substitute placeholders in the copied metadata and body.
6. Write the new file at the target type's folder.

Identity and placement are instance data, so they are not copied. Everything
else is template data, including tags, status and unknown fields, which makes a
template useful without a second configuration syntax.

## Placeholders

| Placeholder | Value |
|---|---|
| `{{id}}` | the new record id |
| `{{title}}` | the requested title, else the template title |
| `{{url}}` | the requested URL, else the template URL |
| `{{host}}` | the URL host |
| `{{slug}}` | the filename slug |
| `{{date}}` | local date, `YYYY-MM-DD` |
| `{{now}}` | UTC RFC 3339 timestamp |

Substitution is one pass of literal replacement, not evaluation. Unknown or
unclosed placeholders are preserved, so a template may contain text that looks
like a placeholder for another tool without being silently eaten. A title that
substitutes to whitespace is stored as absent rather than as an empty string.

## Saving a template

`save_as_template()` clones an existing record into `Records/template/` with
`target_type` set to the source type and a default title of `<title> template`.
The source file is untouched. Templating a template is refused: the outer
template's target would be `template`, which is never what the user meant.

## Limits

- Placeholders read values of the new record only; they cannot reference other
  records or properties yet.
- A template cannot choose a different folder per application; the new record
  starts in its type folder and can be moved.
- There is no inheritance chain. One template produces one record.
