---
topic: ai-connector
type: task
status: deferred
related:
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../crates/lilac-core/src/formula.rs
  - ../../Tasks/active/T11-classification-rules.md
  - ../../Tasks/active/T10-rules-organization.md
---

# T16 — AI connector (local or API key)

## Goal

An optional assistant that can classify a page, fill fields, judge duplicates and
summarise a set. It is a rule source, not a second data path: whatever it
returns still goes through the same preview, validation and save pipeline.

## Requirements

- Two provider shapes:
  - a local embedded model, offline by default;
  - any hosted API selected by the user with an API key stored outside the
    records folder.
- A JEK-like capability set with explicit controls: the model can be asked for a
  judgement, and can request **searches** to gather more information before
  answering. A run has a budget (number of searches, tokens, time) and returns
  its sources.
- Uses:
  - stage A classification (`type`/`subtype`) when rules are not confident;
  - stage B field extraction, including summaries and tags;
  - duplicate judgement for near-matches that URL/title rules cannot settle;
  - post-processing suggestions (never silent writes).
- Results are advisory. They appear as proposed values with a confidence and a
  source; the user or a rule marked `auto` accepts them. Low confidence leaves
  the form open instead of guessing.
- No page text leaves the machine unless the user has enabled a hosted provider;
  local-first is the default and the setting says what is sent.
- Every run is dry-runnable and logged (prompt, provider, duration, result) in a
  way that does not leak the API key.

## Design notes

Treat the model as another evaluator behind the existing rule boundary: it gets
page context and a bounded instruction, returns structured fields, and the host
validates them like any other rule output. That keeps rules, previews and tests
working without a network.

## Done when (not now)

- A local provider can fill a declared field from page context offline.
- A hosted provider can be configured and used with the same result shape.
- A search-then-judge run returns sources and respects its budget.
- Disabling the connector leaves the extension fully functional and silent.

## Open questions

- Which local runtime (embedded GGUF, OS model, CLI) is small enough to ship in
  `orchard.exe` without breaking the single-executable rule?
- Can the model's "searches" be restricted to the user's own records first?
