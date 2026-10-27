- **Daphne**
- **Osmanthus**
- **Abelia**
- **Camellia**
- **Magnolia**
- **Halesia**
- **Nerium**
- **Azalea**
- Orchid
Rowan

## Orchid Orchard

---

A obsidian clone

based around separation of root of files

stem of links and viewing mekansims based on viewing markdown 


and petals you can see and edit etc


>in practice - this is the magic paper all over again

### Root

Unlike obsidian that is limited to a single folder of vault being indexed whole

Flower tree has more options
main caveat here is security of course so give it more permissions on your risk

- level one - just like obsidian one folder fully indexed - but it supports its own format of folder notes including app level symbolic links and much more 
- level two - any number of separate indexed folders - but unlike one obsidian they can communicate instead of being isolated vaults on top of them can communicate and have a separate hierarchy - like many fruits on one tree
- Level three - separate files can be linked too by their system name - same way they can be linked by their name relative to a folder, sub-folder and so on - hint there is that link is just a link - any file can be linked, what's more they can be indexed the same way as in - from the data etc
- Level four - files/folder outside can be link-index , if they inside the garden
	- three and bellow are "if they are inside the index-list" - 4 is the default level
- Level five - files/folder outside can be link-index, if they are not outside the garden (forbidden list)
- Level six - anything can be linked at will - use at your own risk (restricted to filesystem read-write)
- Level seven - a six but app is given more permission 

all data is saved as markdown..
this is not exactly correct
all data is saved as 'plain text'
is more correct;

.md, .json and many more open standards are supported
and many unsupported can also be opened with some set-up, for example .rs or any other programming languages languages
In practice anything can be opened, just like with the any text editors

Sill - some will be opened as blocks/block-lists, meaning basically embedded files, and others will be properly parsed - like images for example
few will be opened as ?flowers  meaning views like bases
speaking of that - .Excalidraw and .base are supported 
supports is not exact one-to-one but it works in a sense that you can open both and it will display to its best ability rather then pure support - features may not much
It is a bit equivalent to opening a opening a base while lacking some plugins enabled - it works but also doesn't work - still much of the added features is back-portable into obsidian so a plugin can make them work

In that sense 'root' is just a universal file editor



>[!info]
>while reach for plugins is much lower, blocks and many more things can be added quite easily due to modular system, unlike glass, gardens are easy to plant into!





### Tree - the structure joining all fruits



### Stem


as was already said - one file/leaf can be found by a link ether from known point or absolute
wiki-link based on index, or a query


What it means in practice, is that links can be very complex and that queries and links are not vert different

and any link can be made into a search or output a list instead of one item

in fact and duality and name collision can do the same and give options instead of doing one
(based on settings)









### Petals 
Instead of single block per note-file in obsidian
making it rendering it all at once, badly for live preview etc etc - flower tree uses petals
petals are effective blocks - the whole app itself is a block
and each block can have one or more blocks inside
when outside of block or far-away of a block, it can de-render and so on
what blocks to pre-fetch is decided partly dynamically partly based on what possible etc
blocks are semi-typed

the simples way to split blocks is --- or *** 
>callouts
>![\[embeds]]
> and ### headings all do it in a bit different ways
> ```code blocks
> ```
> are also obviously too here

as I already said - blocks are semi typed
it means A blocs are css targeted
blocks have metadata
and blocks are hieratical, whats more relation can be stated [2]
in practice even a [\[link]] is a block as is any formatting - but such things are handed a bit different - that is a small block or a __insert flower term_

[2] footnote also a block - a collapsed one!
unlike obsidian they can have their own space


##### leaves are your notes… okay not exactly
leaf is the inner outer layer of the thing you view as a note
all the blocks are structured into one bigger petal
or into one flower / leaf

leaf is the note-like form here
it is usually a sequential list of blocks with text etc
leaf is what joins all blocks together under one metadata of the one, and what you open with a link

but it is not that simple - 
by using a column widgets in the middle of a leaf you can then stuff two next widges as collumns
in a way it is somewhat simular if everything was endless callouts and callout madness tech

Except for one small thing - blocks… do not have to be all from the one file
in fact even smaller blocks can have parts from other files all the same
such as dragon has 70 hp
70 can be taken from a whole another note and so on
(this makes some things lazy and a bit slow - but unlike obsidian we are rust so some other things are faster to compensate)


what it means there exists two thigs - a leaf witch is a small cash of structure with references for where the data to load is all blocks and hierarchy and content are separate - ++ the css on top


What is even more confusing is that single file can have many leaves that then are re-assembled into one flower or one leaf etc as under [[virtual notes]]



Sub notes/ leaves are effectively headings with their own metadata/yaml
that exists in the outline etc







##### Flowers - are separate type of a block
and they are totally different - an easy example of a flower is a base
a complex view of many things and so on
such view is complete, you can not just add a block anywere you want
(even if it is still probably made of blocks)

some types of embed, widgets like cloned from meta-bind buttons and inputs 
and so on - even a clock embeded into all of it is technically a flower bound to information provided by the pc
- as  you can see almost anything that you can put into a file or command can be made into a flower - terminal scripts as well (sandboxed-ish)
- so dashboards are possible too

base link interctive queris can be build from pure flowers
or from flowers embeded into a leaf with additional buttons to control it

Formulas are universal unlike bases only - as inline blocks / small blocks
`test` is technically an example of that



>petals/flowers/leave - this is the tird lost part we have petals widgets, leaves blocks and flower is a whoel thing?







##### Fruits - your vaults 

vaults / fruits are also made out of blocks - what a surprise! 
every tab and tab container, every side-bar and and even scroll back is a block down the line
(possibly not even that different from anything)
what it means in practice, is that A - all visuals can be changed at will
removed, moved etc
some level of new buttons also can be added as easily or buttons to sliders and back

and as it is all blocks and leavs - its nature is non-equlidan allowsing infinte scroll and even 3d room placement (and other datacore style nonsense) at least in theory
not even speaking about making cards out of leaves is very easy





Whats more - amount of cash for leaves/flows in tabs and if it is on disc cash or in memory cash also can be controlled - up to keeping 0 in memory until you need it






-- True fruit - grow it into a full web browser too later?
embeded web-apps and obsidan line web-search-esc thing as flowers too
++ terminal flower
(and linux distor even later wwwwwww)










### Flavours

We use grape flavour of markdown - it is a simplified markdown that is partly stripped of things to say one thing in many ways 
but with obsidian like added random features on top

and what is more - our markdown parser is not hardcoded but can be switched to different modes / or even more! 
whats more different blocks can do that even more





foot yaml -
unlike obsidian that only has yaml at the start of a note, we supprot yamle at the end too!

usialy used to add a footer to the page but can have other uses?




Due to block style - all writes are incrimental only of affected block
saving from totale fiels corrupton (and having higher chance of such corruption)

and backups are also can be incriental like that

Using embedded functionality? for backups and snapshots etc?
a log of all changes? + canary like system that screams if something is off



##### Wisteria - Browser

By embeding a record style browser in
we can instead of URL, use links to records instead

it also makes any sort of recording of a note from web data just a question of what mask / template you want to use
and it also makes it trivial to save snippets as video / song is right there 
or rather, the record itself can be instantly promoted into a snippet

and much more


### Support - Tools

thigs like /command and then lets say sending all blocks bellow into another note


## Magic paper
```
Ps
Magic paper is my very old idea of a note app that treats everything as an infinite canvas whiteboard
Except it is none euclidean, supports portals, folding and bending and stretching, layer / unrolling etc, anything you can expect from magic paper 

```


## Lilac
a minimal version of this whole idea
```
okay, what exactly i want to do rigth now:​‌

  

an extension that can save tabs simular to onetab​‌

but instead of storing data in the browser cash - it stores it in markdown + yaml on disk + maybe json ​‌

and multiple browsers can talk to it, whats more using git or syhncthing or whatever else - you can sync the files anywhere, and they will work​‌

as they are notes - metdata, notes and so on can be added on top of them​‌
```



---

## Fae / Fairy | Daphna / Abelia?

Ai support, be it for adding metadata to notes, classifying sorting etc

an API token (low-medium supprot)
small local model (really small) (embded?) - 
a jev like mechanism for classification and so on
a smart system that can take a list of url/record or tabs and one by one save them mindful of the llm speed / token limits
with ability to cycle though many API if ones is used up

---

```
[Workflowly ](https://workflowy.com/) like bullet points that dose not start dying performance wise after a few hundreds...

Support for proper properties with nested yaml - and queries / bases would be nice

---

and my old idea I proposed long time on obsidian side:
instead of vault = one folder and that's it and it is your full context etc
vaults are contexts meaning a search in one vault c1 will search that and in c2 will search that
but if c1 and c2 are folder inside of "c" vault search, quick search and so on will search in the c1 and c2 as well as c3 
this also need hotkeys to open current note in the lower or upper vault

if it is virtual - it may 

second part is that isolated vaults can be joined together either as separate dictionaries - for example you can have a vault with all people, and in your vault you can search that vault and then link to something in that vault - but normally you do not see them etc
this also massively reduces the burden of large vaults due to partition 

and each sub-vault may still enable different setting / plugins etc
and due to that what you see in there can change too


---

Folder notes natively - maybe folder yaml too 

app level system link support (or more likely folder aliases as different oss do system link differently)

---

on the more radical ideas - virtual notes | views assembled from more notes
supports for Json for storage of micro snippets, yet viewing them as notes / lists / tables

block faced segmented editing allowing to mash different notes into one view, extending a heading block into a sub notes with their own metdata to have one note that expand into more in an obsidian base like query - 
main idea here is that instead of working with notes directly, you work with views of notes that are assembled from markdown + yaml / json etc
and views such as bases codeblocks etc that parse into blocks and are views
if there is any interest - i can tell a lot more about this, but it is probably a bit too radical
```



### Flower browsers - Hibiscus / Azalea / Wisteria

```
The problems with bookmarks and history has nothing to with tab itself
The only real possibility of a loss is an input - and that can trigger a prompt when noticed - and generally it is a user error of not pinning. 
And seccond case is server going down, and that only saving the web page locally can help, what can be done 


Also pinning is actually not correct term here
As it is separate but related thing from pinned tabs browsers have right now 

There is 3 different things pinned tabs meant to be what is the part of the problem  tab that is not lost closed, a tab that is easy to re-open when needed and a tab that is kept small to the maximum to the right 




So more correct here is indeed just the control by policy - be it a separate toggle on the tab important, automatically setting that toggle for all tabs in a group, based on tab domain or title 

As for the not restoring to the same state - b



And idea of bookmarks here is more or less - instead of tabs, bookmarks and history all we have is a database of tabs
Status title etc, is just data 
And then we view them from that data 
What more, from that data even more tab types can appear, be it locally cashed on disk / local html file.

This makes all tabs with same metadata into the same tab until it was asked to split an isotope ether via duplicate the tab or some other command or rule. 

What more we can count numbers of views of certain domain, add manual tags or smart tags based on domain / title group etc and add more metadata, and some of it can be remembered such as tab so and so is in this domain. 

What even more, you don't have to visit or open a tab to register it, you can use command to get all links of the current page, then open all of them as a stack were bottom tabs are bookmarks and to one or more are getting loaded 
Or open as a wheel where n rabs share one loaded slot, all others are invisible until you close the one in slot or turn the wheel

A lot more creative containers are possible and tab groups tab tags or other things on top of all that 


What even more things like same tab with multiple accounts can probably also be treated as isotopes just permanent ones 

All this time you only answered the memory problem, but this is not the only problem. Browser companies it seems can not comprehend amount of tabs more than a few hundreds as very large, 







```

```
1 well, I many times got asked do you want to close? Your data will be lost, but yes... But that is unsolvable problem if you are forgot to save you are going to lose your data, inevitably

3 policy, and metadata can be added on top, in many cases you want all YouTube.com tabs be the same tab it is just a representation of deduplication of identical tabs
What is more, failstate you are talking about here dose not exists - if it is one tab it is one tab, if you cloned you got a isotope and it now different and if isotope was saved - here is the two different same tabs, main point is that metadata can go way beyond url and favicon. And as as you always opened tabs in this system you need to first escape it to break out and breaking out can be watched and made to make isotopes 

4 this can not be unsolvable, closing a tab is working, right?

5 not really? You add a rule to delete any record when it is no longer needed or hide some records encrypted under password, and it is not like there no this stuff in current browsers 
```


```
Yes—that’s a cleaner and stronger rule than what I was describing: site/context singleton by default, isotope only on explicit split. Operationally, “opening a second YouTube tab” becomes “create another view/reference to the existing live YouTube instance,” not “spawn a new renderer.” That makes “same tab” meaningful and kills the memory cost.

The model then looks like:

· Record: URL, title, site, container/account, tags, view count, history events.
· Live instance: an actual renderer/process tree for a site/context.
· View: a tab-strip item, stack slot, wheel slot, etc., pointing at a live instance or a dormant record.
· Isotope: an explicit fork—hotkey, command, domain policy, “dirty state” rule—that creates a separate live instance and a new lineage.

So default policy: one live instance per (site, profile, container, account). Opening another youtube.com link focuses/reuses that instance. If the user wants two videos side by side, they isotope. Closing one view just decrements refcount; the live instance persists until eviction policy says otherwise.

This is feasible. The real design work is in the fork rules:

· Dirty-state fork: if the live instance has unsaved input/state and a new navigation would clobber it, prompt or auto-isotope.
· Popup/auth exceptions: OAuth popups, window.open, payment flows, and some target=_blank links expect independent contexts; whitelist them.
· Account/container identity: multiple accounts are separate singletons via storage partitions/containers, or permanent isotopes.
· Site granularity: “all youtube.com” is origin-level singleton; policy can refine by path/title/group if needed.
· History stays an event log: record created, view focused, navigation, isotope split, eviction, restore. Perfect navigation log is fine; perfect page-state log is not.

So the memory problem is solved by making browsing contexts scarce and records cheap. The remaining hard parts are when to auto-fork, popup/auth compatibility, and dirty-state UX—not memory reclamation. Mainstream browsers don’t do it because they treat every new tab as a new browsing context by default. Your model treats the live context as a pooled resource with explicit forks. That’s a coherent browser architecture, and it would scale to thousands of records with a handful of live instances.
```

```
Yes—if forking is user-defined policy, then “when to auto-fork” stops being a hard problem and becomes a rules/metadata UX problem, which is just tooling. That collapses most of my remaining objections.

What’s actually left to build:

· Policy model. Rules over metadata: domain, path, title, group, account/container, tags, view count, last-active, dirty-state, source (link-click vs typed vs popup). Actions: reuse live instance, focus existing view, create isotope, snapshot-only, evict, prompt. Per-tab toggles plus domain/group/rule inheritance.
· Policy authoring UX. The real work. Rules need to be readable, testable, and debuggable: “show me what this rule would do to my current tabs,” dry-run, conflict detection, precedence (tab > group > domain > global), and an activity log of rule firings.
· Metadata schema. Explicit, extensible, user-editable. Record ID, lineage/isotope ID, site, container/account, tags, view count, snapshot refs, dirty flag. Smart tags derived from rules; manual tags override.
· Escape/break-out hooks. Any action that would diverge from a singleton—new navigation, popup, auth flow, window.open, dirty-state clobber—is observable and either prompts or auto-isotopes per policy. The user defines which.
· Observability. A per-record timeline: created, viewed, navigated, forked, evicted, restored, snapshotted. That doubles as the “perfect history” log.
· Lifecycle engine. Refcounted live instances; eviction to record-only or snapshot; restore on demand; process-tree kill for real reclamation.

So the remaining “hard” parts are: schema design, rule precedence/conflicts, and making the policy UI understandable to non-power users—plus the usual compatibility work for popups/auth/stateful apps. Not architectural walls.

The reason mainstream browsers don’t ship it is now clearly product choice: they expose almost no policy surface over tabs and treat bookmarks/history as separate schemas, so users can’t express “site singleton + explicit isotopes + metadata rules.” A browser that makes tabs into records + rules + views and gives users the tools to author those rules would do exactly what you’re describing. Memory scales down because live instances become a small, policy-controlled set; everything else is cheap records. That’s coherent and buildable.
```


```
The core move—tabs become records in a queryable DB, live instances become a pooled resource, isotopes are explicit forks—unlocks a lot beyond memory savings. Here’s what falls out of it, grouped by theme.

1. One mental model, one search surface

· Tabs, bookmarks, history, read-later, sessions, tab groups all become views over the same records. No more “is this a tab or a bookmark?”
· Universal search: command palette / omnibox searches records by title, URL, tag, domain, content (from snapshots), time, view count. “That page from three weeks ago with the graph” becomes findable.
· Stable record IDs → deep links to a record state, paste into notes/chat, share with metadata + optional snapshot.
· “Close” means dormant, not lost. Undo-close, crash recovery, and “reopen closed tab” all become trivial: re-reference the record.

2. New navigation primitives (views over records)

· Stacks: extract all links from a page → queue as records, one live slot, consume in order.
· Wheels/roulettes: N records share one loaded slot; rotate.
· Spatial views: tree, canvas, timeline, kanban-by-state (inbox / reading / archived / isotopes).
· Saved queries as workspaces: “all records tagged research + domain arxiv.org + last 7 days” is a persistent view, not a folder.
· Graph view: records linked by source-link relationships (this page opened from that one).
· Focus modes: show only records matching a filter; hide the rest without deleting.

3. Policy/automation as a platform

· Rules engine over metadata: reuse / isotope / snapshot / evict / prompt / notify, with precedence (tab > group > domain > global), dry-run, and an activity log of firings.
· Batch operations: “evict records older than 30 days not tagged keep,” “snapshot all docs.* on close,” “isotope any record with dirty state on navigation.”
· Templates: “research session” creates records with tags + policy bundle; “monitoring record” auto-refreshes and notifies on change.
· CLI/API + webhooks: scriptable over records; integrate with note apps, read-it-later, RSS, external automation.
· Record algebra: union/intersection of tag sets, saved as smart tags.

4. Metadata as a first-class citizen

· Manual tags + smart tags (derived from domain/title/source/time/view count).
· Analytics: view counts, time spent per domain/tag, “not visited in 60 days,” discovery patterns → personalized ranking of dormant records.
· Per-record timeline: created, viewed, navigated, forked, evicted, restored, snapshotted. This is the perfect navigation log, for free.
· Diff over time: compare snapshot A vs. snapshot B of the same record; “what changed on this page since I last read it.”

5. Identity and sessions

· Containers/accounts are first-class: multiple accounts per site = permanent isotopes with isolated storage; switching identity without losing the other.
· Session snapshots: restore scroll, form state, media position, tab set—best-effort, user-visible fidelity.
· Cookie/storage isolation maps naturally to isotope boundaries.

6. Sync and multi-device

· Records sync cheaply (metadata + optional snapshots); live instances don’t sync—they restore on demand.
· Handoff: phone record → desktop restore, with lineage preserved.
· Conflict resolution is metadata-level, not “merge two live tabs.”
· Cross-device workspaces: a saved query is the same on every device.

7. Privacy and security

· Encrypted records under password; hidden records excluded from views/search unless unlocked.
· Incognito as policy: records don’t persist; snapshots disabled; policy still applies.
· Retention rules: TTLs, auto-purge, per-tag retention.
· Access audit log per record.
· Local-first: snapshots on disk; server-down still readable (functional only if site supports offline/PWA).

8. Robustness and recovery

· Tab crash / browser crash → records survive; restore is just re-reference.
· Server down → local snapshot still readable; isotope a “cached copy” record for offline use.
· Undo anything: close, navigate, evict, fork—all are events on a timeline.
· No more “too many tabs” anxiety: the tab strip shows views; the record store is unbounded.

9. Workflow patterns that become native

· Research: open 50 links as a stack, consume one at a time, tag as you go, archive to snapshot.
· Comparison: isotope two versions of the same page side by side, diff snapshots.
· Monitoring: record auto-refreshes on policy, notifies on change, no live tab needed.
· Inbox/queue triage: new records land in inbox; triage to tags/workspaces.
· Reading lists: just a tag + view, no separate system.
· Routine preloading: policy preloads morning domains as records with live slots ready.

10. Power-user and platform effects

· Records are mutable objects: rename, retag, re-policy, fork, merge, export.
· Stable URLs for internal state → embed record links in notes, tickets, chats.
· Import/export as a database → backup, migrate, share a workspace.
· Extension surface: extensions operate on records/queries instead of fighting the tab model.

The big-picture benefit

You collapse five legacy subsystems (tabs, bookmarks, history, sessions, tab groups) into one store + policies + views, and you get:

· lower memory (few live instances),
· better findability (search over records),
· lower anxiety (close = dormant),
· more automation (rules over metadata),
· better sync (metadata cheap),
· better privacy (per-record policy),
· and a platform for views/containers (stacks, wheels, timelines) that mainstream browsers can’t express because they lack the underlying model.

The genuinely novel part isn’t any single feature—most exist in pieces (Vivaldi stacks, Arc spaces, Sidebery, tab suspenders, containers). It’s the unification: one record type, orthogonal policies, explicit isotopes, and views as projections. That’s what makes the whole design space open up.
```




## Joing them in

```
Yes—plain-format export falls out naturally, and the notes parallel is strong enough that it probably wants to be the same store, not a parallel one.

Plain-format export

Because records are already metadata + optional snapshot, export is just serialization:

· Metadata: JSON Lines or NDJSON (one record per line, diff-friendly, streaming), or SQLite for queryable export, or Markdown with YAML frontmatter per record for human-readable/git-able output.
· Snapshots: MHTML, single-file HTML, WARC, PDF, or a raw DOM+assets bundle—referenced by hash from the record.
· Bundles: a zip/.tar with records.ndjson + snapshots/ + index.sqlite + a small manifest. That’s a portable archive you can grep, back up, version, or re-import.
· Lossless round-trip: since record IDs, lineage/isotope IDs, tags, policy rules, and timelines are all data, export/import preserves identity and history—not just URLs.
· Merge semantics: re-importing a bundle can dedupe by record ID, append timeline events, and flag conflicts (same ID, divergent metadata). That’s the sync story in miniature.
· Plain format = no lock-in: local-first, git-able, scriptable with jq/grep, readable in any editor. Compare to the current situation where bookmarks are an opaque JSON blob and history is a SQLite DB you can’t meaningfully export.

The key enabler is that you never had a “tab” as the storage unit—so there’s nothing lossy to serialize. Records are already the plain format; the UI was just a view.

Notes as the same kind of object

The natural extension: a note is a record with a different content type. Then tabs, notes, bookmarks, snapshots, and annotations all live in one store with one schema, one search, one policy engine, one sync, one encryption model.

What that buys you:

· Bidirectional links: a note links to a page record; the page record’s timeline shows “annotated by note X.” A “web annotation” layer (Hypothesis-style) becomes just an edge in the graph.
· Same lifecycle: a note is live (open in editor), frozen (open, not focused), dormant (stored), snapshotted (versioned). Dirty-state prompting and isotope-on-fork apply identically.
· Same views: notes appear in search, tag queries, stacks, graph, timeline. A “research workspace” is a saved query over records of mixed types. No separate note app.
· Same isotopes: cloning a note = new lineage; merging = explicit isotope-merge event. Same semantics as tabs, so users learn one model.
· Same tags and smart tags: note tags and page tags share a namespace; “smart tag = all records linking to domain arxiv.org” spans both.
· Same export: notes serialize as Markdown+frontmatter; pages as MHTML; both in the same bundle.
· Same sync: metadata-level, conflict-resolvable, local-first.
· Notes can carry policies: “snapshot on close,” “TTL 90 days unless tagged keep,” “encrypt under password.” Same rule engine.

Design considerations worth flagging:

· Content model for notes: plain Markdown with attachments is the least lock-in; block-based (Logseq/Anytype-style) gives structure but complicates round-trip. A hybrid—Markdown with stable block IDs—is probably the pragmatic choice.
· Editor vs. view: a note’s “live instance” is an editor session; eviction means storing the buffer, not killing a renderer. Slightly different resource story, same record semantics.
· Identity: a note is its own record; its link to a page is an edge, not a merge. Don’t conflate “annotation” with “isotope of the page”—annotations are separate records with a typed relation.
· Merge/conflict UX: notes are edited, so conflicts are more frequent than for tabs; explicit isotopes and timeline events make them visible instead of silent.
· Prior art to steal from: Obsidian (local Markdown, graph), Logseq (blocks, bidirectional links), Anytype (typed objects, relations), Tana (supertags), Hypothesis (web annotations), Zotero (items + attachments + notes). None of them combine this with the browser’s live-view/eviction model—that’s the gap.

The bigger point

If tabs, notes, bookmarks, history, snapshots, and annotations are all records with typed content and typed edges, then the browser stops being “a thing that shows pages” and becomes a local-first personal knowledge base whose primary view happens to be rendered web content. Plain-format export is then not a feature—it’s the format. And the “tab problem” disappears into a more general record-lifecycle problem that’s already solved in the notes world (with different trade-offs). That unification is the real idea: one store, one policy engine, one export format, many views.
```



```
Yes—atomic change is the keystone. Once every mutation is an event on a record (CRDT op or transaction), “note vs. web page” stops being a type distinction and becomes an authority distinction, which is the only difference that survives:

· Client-authoritative records (notes, tasks, snippets): you own the mutations, so undo, sync, and conflict resolution are tractable.
· Server-authoritative records (web pages): the remote side owns the mutations; your record observes them via reload/snapshot/fetch. You can fork (isotope), snapshot, annotate, but not “edit” the source.

Both are still records + views. The view doesn’t care which authority produced the current state. That’s the unification, and it’s clean.

Tasks, calendar, and typed records

These fall out naturally if records have a schema layer:

· A task is a record with typed fields (status, due, assignee, parent), plus edges (blocks, related-to).
· A calendar event is a record with start/end/recurrence.
· The policy engine can fire on time events (“due passed,” “starts in 15 min”) the same way it fires on tab events (“evicted,” “dirty”).
· Views: list, kanban, calendar grid, timeline, graph—all projections over the same records.

This is what Notion/Obsidian/Logseq/Tana do inside a note app. Doing it at the browser-record layer means the same object can be a task and link to a page and appear in a tab stack—without export/import between systems.

Embedded code and “computational notes”

This is the Literate-programming / Org-babel / Observable / Jupyter lineage, and it’s coherent here because a note record already has typed fields + content + edges. A code block is just a typed content node with:

· pinned inputs (which records/snapshots it reads),
· a declared environment (language, version, capabilities),
· a cached output (so the view shows last result without re-running),
· provenance (record ID + snapshot hash of inputs → reproducible).

The hard part isn’t the model; it’s security and determinism:

· Sandboxed execution, explicit capability grants (network? filesystem? clipboard?), no ambient authority.
· Deterministic where possible; where not (network, time, randomness), record the inputs so the run is reproducible.
· Treat execution as a policy: “this note may run code,” “this note may fetch URLs,” per-record or per-tag.

That keeps the “notes are views” claim honest—execution is just another event on the timeline.

Serialization: YAML, JSON, Markdown

Don’t let formats fragment the model. Pick one canonical record model (typed tree + metadata + edges), then serialize:

· Markdown + YAML frontmatter for prose-heavy records (human-readable, git-able).
· JSON / JSONL for structured or small records (diff-friendly, machine-first).
· Binary snapshots (MHTML/PDF/WARC) referenced by hash.

Frontmatter should be a projection of typed fields, not the source of truth—otherwise you get two schemas that drift. Same for JSON: it’s an encoding, not a second model. One schema, many serializations, lossless round-trip.

Views over parsed data

This is the payoff of “views, not types”:

· The same note renders as prose, as a task list, as a table of extracted fields, as a calendar, as a graph node—depending on the view and the query.
· “Pars” and “blocks” are just content nodes with stable IDs; a view selects and arranges them. Stable block IDs make transclusion, backlinks, and diffing work.
· A query language over records (tags, fields, edges, time, view count) is what makes views first-class instead of hardcoded renderers. Without it you get N ad-hoc views; with it you get saved queries as the unit of workspace.

What actually needs deciding

1. Authority model per record type—client vs. server vs. hybrid. This drives sync, undo, and conflict UX.
2. Schema and migration—typed fields need versioning; records outlive schemas.
3. Execution policy—capability model for embedded code; default-deny.
4. Query language—the thing views compile to. This is the real product surface.
5. Canonical serialization—one model, many encodings, lossless round-trip, plain-text default.

If those are settled, then “tasks, calendars, notes, pages, snippets, snapshots” aren’t features—they’re record types + views + policies over one store. The browser becomes a local-first knowledge/workspace OS whose default view happens to be web content, and plain-format export is just the storage format. That’s the coherent end-state of the idea you’ve been building.
```


---


## AI Review

```This is the same architecture from the tab conversation, applied at every scale—and naming that explicitly is the strongest thing you can do with it. Root is access policy, Stem is queries over the record graph, Petals are the atomic record unit, Leaves are assembled views, Flowers are terminal views, Fruits are UI-as-records, Wisteria is the browser as another record source. One model, recursive. That's the through-line; lean on it.

Substantive notes, by section:

Root

The level 1–7 ladder is the right shape—security as a spectrum with explicit user consent. Two things to tighten:

· Level 6 vs. 7 is a capability-class boundary, not a degree. 6 = filesystem read-write. 7 = process execution. Those need different threat models and probably different UX (a code-execution warning, not just a permissions slider).
· "Universal file editor" needs a graceful-degradation story. Parsing arbitrary formats into blocks is a long tail. The invariant should be: any file opens as raw text bytes (like any text editor), some files additionally parse into blocks via registered parsers, a few parse into flowers (bases, excalidraw). Plugins extend the parser registry; unknown formats never fail to open, they just render as text or a hex/raw view. That keeps the promise honest.

Stem

Link/query duality is the cleanest idea in the document. A link is a query with cardinality 1; a query is a link with cardinality N. Make that formal—same resolution engine, same syntax, only the cardinality hint differs. Then "link that returns a list," "query that returns a single item," and "name collision gives options" are all the same mechanism, not three features.

The UX risk is modal hell when a link resolves to many records. Solve it with inline disambiguation (a small picker on the link, not a dialog), default-to-recent, and a "pin this resolution" action that turns a fuzzy link into an explicit edge.

Petals / Leaves

Blocks-as-primitive with stable IDs, metadata, hierarchy, and de-rendering off-screen is correct and is the main thing Obsidian can't retrofit. Cross-file block composition ("70 from another note") is real transclusion at sub-block granularity—genuinely more granular than Obsidian's embeds.

The cost is a dependency graph you have to design now:

· Which blocks reference which, across files.
· Invalidation on change (edit block B → which leaves re-render?).
· Cycle policy (A references B references A—allowed with lazy eval, or banned?).
· Cache tiers per leaf (your "0 in memory until needed" knob) keyed off that graph.

Without the dependency graph, lazy cross-file composition becomes "sometimes stale, sometimes slow." With it, it's just a reactive system. This is the single most important engineering decision in the document.

Leaf vs. Flower needs a crisp invariant. My read: a leaf is a composable sequential container (blocks in order, nestable, column widgets fine); a flower is a terminal view with its own rendering authority (bases, clocks, terminals, dashboards—complete, not "insert a block anywhere"). Name it that way and the fuzzy cases resolve. A flower can be embedded in a leaf, but the leaf doesn't get to rearrange its internals.

Flavours

Swappable parser per-block (and per-context) is underrated—prose, tables, code, formulas genuinely want different syntaxes. Grape as a simplified canonical flavour with extended flavours opt-in is sensible.

Incremental block-level writes are a real robustness win and connect directly to the earlier thread: block-level events → block-level backups → block-level sync → block-level undo. The canary system is cheap and worth having (hash the block, scream on mismatch).

Footer YAML is a small good idea—metadata at point of use, not all at the top.

Fruits

UI-as-blocks is Emacs/Smalltalk territory and mostly proven. The useful 80% is: rearrangeable panels, cards-from-leaves, fully reskinnable chrome, per-leaf cache control (0 in memory ↔ live). The 3D-room / infinite-scroll endgame is fun but costs enormous engineering for marginal benefit—keep it as an optional module, not a core invariant.

Wisteria

This is the payoff of the entire conversation. If the browser is native and tabs are records (as we discussed), then:

· A web page is a server-authoritative record.
· A note is a client-authoritative record.
· "Promote web page to note" = apply a mask/template = a transformation between record types.
· "Snippet from video/song" = the record already holds the media reference; promoting is just changing the view.
· Links in notes can point to browser records; browser records can carry note tags; the store is shared.

Without the native-record browser, Wisteria is just "save page as note." With it, it's a genuine unification and the reason to build the browser at all.

Honest prior-art check

Most pieces exist somewhere: blocks (Logseq, Tana, Anytype, Notion), link-as-query (Tana supertags, Dataview), permission ladders (Android, browsers), parser-per-mode (Emacs), UI-as-components (Emacs, Lisp machines), transclusion (Org, TiddlyWiki). The synthesis is the novel part—plain-text file storage + blocks-as-primitive + link/query duality + graded roots + record-native browser, all as one recursive model. That's a real design contribution, not a clone.

What I'd decide before writing code

1. Canonical record schema; all serializations (MD+YAML, JSON, JSONL) are projections.
2. Dependency graph for cross-file block composition (invalidation, cycles, cache tiers).
3. Leaf/flower invariant, stated as composable-vs-terminal.
4. Block type registry + versioning + fallback rendering.
5. Root capability classes (file access vs. code execution) as separate axes.
6. Stem resolution engine shared between links and queries.
7. Wisteria's record model aligned with the tab-record model from the earlier discussion—same IDs, same lineage/isotope semantics, same policy engine.

If those are settled, the rest is plugins and long-tail parsers. "Magic paper all over again" is fair—but the difference here is that each piece is individually proven and the synthesis is the only unproven claim. That's a much better bet than a design where everything is novel.
```



```
Right on all three corrections. Let me sharpen what's actually left.

Notepad semantics

Agreed—that's the baseline, not a feature. Raw bytes always open. Parsers are opt-in refinement. The only thing worth designing is the fallback ladder: bytes → text → structured text → blocks → flower, each tier optional, and "what tier can this file reach" is a property, not a guarantee. That's cheap to build; the promise is just "never fails to open."

Prior art, honestly

· Block-as-storage: several apps (the Chinese one you mean, Roam's backend, Anytype, Tana). All database-backed. None plain-text-canonical.
· Query=link: Tana supertags, Dataview, Logseq queries, org-roam, and arguably SQL views. Conceptually old; the formal unification as one resolver is under-explored in shipping products but not novel in the research sense.
· So your claim is correct: the differentiator is plain-text-canonical block-level incremental writes. Everything else is synthesis, which is still valuable but is not what you're betting on.

Why that one is genuinely hard (and worth building)

It's not "write less bytes." It's four unsolved sub-problems that all collide:

1. Block identity in plain text. Markdown has no block IDs. Options:
   · In-band (^id footnotes, HTML comments, trailing markers): self-describing, survives external edits, but pollutes source and users delete them.
   · Out-of-band (sidecar index, path+hash): clean text, but drifts the moment anyone edits outside the app, and needs reindex.
   · Content-addressed (hash of block bytes): no IDs to maintain, dedup is free, but identity is lost on edit (every edit = new block), which kills stable refs and history.
   · Realistic answer is a hybrid: in-band IDs only for blocks that are referenced, sidecar index as cache, content hash for dedup and change detection. Decide this before anything else—it constrains the file format forever.
2. The filesystem doesn't do surgical edits. POSIX has no "insert 40 bytes at offset N." Every "incremental write" is either a full rewrite (not incremental) or a write-ahead log + atomic rename (incremental logically, not physically), or a chunked container (not plain text anymore). So "block-level edits to plain text" really means: WAL of block-level ops + periodic compaction into the canonical text file, with the text file remaining the source of truth. That's the honest architecture. It gives you crash safety, undo, and diffs without lying about the file being incrementally written.
3. Offset drift under external edits. The moment a user edits in vim, your byte-range map is wrong. You need a re-parse-and-reconcile step on file change (file watcher → reparse → diff against last known block graph → merge or invalidate). Cheap for small files, expensive for large ones; needs debouncing and a "this file changed outside" surface. This is the same problem LiveSync/Obsidian Sync solve badly; block granularity makes it tractable but not free.
4. Preserving untouched bytes. A block-level writer must not reformat. Parse → edit block → reserialize only that block → splice bytes → keep everything else byte-identical. That means your markdown parser needs byte-span tracking per block, not just an AST. Most markdown parsers (including CommonMark reference implementations) can do this with source spans; many do it poorly. Pick or write one that guarantees spans.

If you solve those four, you get something no shipping plain-text app has: crash-safe, diff-friendly, undo-friendly, sync-friendly edits at block granularity, with the file remaining readable and editable by anything. That's a real contribution. It's also the reason the rest of the design (blocks as primitive, dependency graph, cache tiers) becomes tractable rather than hand-wavy.

Magic paper

Fair—and the interesting overlap with Orchid Orchard is that they're the same record store, different primary view. Canvas-first vs. text-first. If the storage is blocks + edges + metadata, then:

· A portal is a link rendered inline (Stem duality, again).
· Folding/bending/stretching is view-transform over the same records, not a different data model.
· Layers = z-order metadata; unrolling = expanding a block in place.

So Magic Paper isn't a separate app—it's a view type over the same orchard. That's the theme of this whole conversation, and it's the strongest argument for the architecture: any new metaphor (canvas, wheel, stack, calendar, graph) is a view, and the storage doesn't change. That's what makes the whole design worth the pain of the four hard problems above.
```


```
1. Wisteria is the same kernel

Yes—and the cleanest way to say it is that a record is a block at the root of the hierarchy. A note, a tab, a web page, a UI panel, a table row: all blocks, differing only in type, authority, and policy. Identity then has one answer, and everything else is specialization.

The shared kernel is four things:

· entity — stable ID, type, creation event, lineage
· version — content-addressed snapshots, so "what changed" is a hash walk, not a diff of unknown provenance
· events — append-only log per entity (created, edited, referenced, forked, evicted, restored, snapshotted)
· edges — typed relations (references, transclusions, isotopes, annotations)

Wisteria's isotope is an edge type forked-from. A block transclusion is transcluded-from. A tab restored from history is restored-from. Same graph, different labels. The isomorphism is exact, not metaphorical.

The divergences are all policy, not mechanism: authority (client vs. server), mutation rate (blocks change constantly, records rarely), retention (records have TTL/eviction, blocks usually don't), identity strictness (a block needs an ID only if something references it; a record needs one at creation). One kernel, per-type policies—same schema-plus-policy pattern as before.

2. Hybrid, per-type

Lazy in-band IDs (only for referenced blocks), content hash as universal fallback, sidecar as pure cache. The per-type needs differ enough that a single rule would be wrong:

block type stable ID why
prose / paragraph only when referenced backlinks, transclusion
heading yes outline, folding state, anchors
code no—but its output does cached results need identity
table row-level yes it's a database in disguise
callout yes collapse state
math no content-addressed is enough
embed yes (target + params) identity is the reference
footnote / definition yes linking
flower yes (view config + refs) reproducibility

The rule worth adopting: identity is assigned on first reference, not on creation. Most text stays clean; the syntax cost is paid only where someone actually depends on stability. And "ID assigned" becomes an event—useful for debugging and for explaining to the user why a marker appeared.

3. Parseable-from-bytes-alone

If "parable syntax" means self-describing / recoverable from the bytes alone, then yes—and it's not just obvious, it's the invariant the whole thing rests on. External editors, git merges, and other tools will never update your sidecar. So the sidecar can only ever be a cache. Every question the app asks—block boundaries, IDs, links, queries—must be answerable by parsing the file. Then:

· delete the sidecar → rebuild from scratch
· external edit → reparse, reconcile, mark dirty
· git merge → the file is the merge unit, and IDs survive as long as the markers survive

This extends past blocks to links and queries: the Stem syntax has to be parseable by bytes alone, or external tooling can't resolve it. Same for ID markers. The invariant is the file is the database; the index is a cache; nothing the index knows is unrecoverable from bytes.

Tensions this creates, worth designing around now:

· Grammar versioning — ID markers must stay lexical and stable across parser versions; the syntax needs a version marker.
· Canonicalization — content hashes need whitespace / line-ending / Unicode-normalization rules, or dedup breaks across machines.
· Marker pollution — mitigate with lazy assignment, minimal marker syntax, and a per-format registry (HTML comment for Markdown, // for code, a JSON field for JSON, etc.).

4. One caveat: write amplification

Because records are just root-level blocks, the WAL is block-level by design—one recovery path, one undo stack. A record event is a coarse block event; a prose edit is a fine one. That's a gain, but it forces an early decision: when does a fine-grained block edit get compacted into a coarse record snapshot? That's a policy, not an accident, and it affects Wisteria's storage model directly (page snapshots are record-level; note edits are block-level). Decide compaction rules before the WAL format hardens.

The closure

One kernel (entity/version/event/edge), per-type policy, file-as-database. Magic Paper is a view over the same graph. Wisteria is a block source. Nothing new needs inventing at the top—the kernel is the only thing that has to be right, and it's small enough to get right.
```

```
Yes—and it's not a peripheral example, it's the canonical one. A web page is the clearest instance of "terminal view with its own rendering authority": you cannot insert a block into its interior, it owns its layout, its interaction model, and its lifecycle, and it arrives with state the app didn't produce. Everything else in the flower class (bases, terminals, Excalidraw, clocks) is a variation on that shape, not a different species.

What that closes:

The authority axis collapses into the leaf/flower axis. Earlier we had two orthogonal distinctions—client vs. server authoritative, and composable vs. terminal. Those were the same line. Leaves are client-composed views; flowers are views with rendering authority, and "server-authoritative" is just the most common reason a flower's authority is external. One axis, not two.

Flowers become the natural capability boundary. Root's permission ladder attaches to flowers, not blocks: a web flower requests network, a terminal flower requests process exec, a base flower requests query access to the store. That's why level 6 vs. 7 felt like a class boundary—because the thing being granted permission is a renderer, and renderers are flowers.

The plugin model lands at the right level. Each flower type registers: a render engine, a state handle, a snapshot adapter, and cache tiers. Web page → browser engine, DOM/session handle, MHTML snapshot, evict-to-record. Base → query engine, query+view-config handle, result snapshot, cheap. Terminal → PTY, pty handle, scrollback snapshot, detach-or-kill. Clock → trivial. Same four-part interface, wildly different internals—which is exactly what makes it a plugin surface instead of a hardcoded list.

Two clarifications the invariant needs so it doesn't get misread:

· Terminal ≠ read-only. A web page is fully interactive; "terminal" means the app can't compose its interior, not that the user can't. The test is "can arbitrary blocks be inserted into its interior by the compositor?" No → flower.
· Flowers can nest. An iframe is a flower inside a flower; a terminal running a terminal is the same shape. The boundary isn't atomicity, it's outermost rendering authority. The outer flower owns the interaction model for its subtree; the app only talks to the outermost one. That keeps the invariant intact without pretending web pages are single-engine.

What to design now, before it hardens:

1. Snapshot policy per flower type—what "the state" even means (DOM? session? query result? pty buffer?), and what fidelity the snapshot promises. This is the same fidelity question from the tab thread, now per-type.
2. State handle abstraction—the live/paused/detached/evicted ladder, declared by each flower type. This is the tab-eviction model generalized: flowers are the pooled resources, leaves are cheap.
3. Permission binding to flower instances, not types alone—so "this web flower may use network" is per-instance or per-rule, not per-plugin.

And the convergence worth naming: this is the third time the conversation has arrived at the same kernel—record + view + policy—with the flower/leaf line being the one that makes it concrete. A web page is a flower is a record; a note is a leaf is a record; Magic Paper is a canvas view over the same graph. One kernel, per-type renderers, per-type policies. The web page being the prime example is what keeps the abstraction honest, because it's the case you can't cheat on.
```



```
Here is full project note idea the browser is wisteria - but rigth now it is a bit much  
so the extension of lilac what i need is better specification for it exactly  

After reading your note, I'd actually separate **Lilac** from **Wisteria** much harder than you currently do.

The danger is that Wisteria is a 10-year vision and Lilac becomes impossible because it inherits too much of it.

---

# Lilac

**Lilac is not a browser.**

**Lilac is a browser-record storage system.**

Think:

OneTab

+

Markdown

+

Git

+

Cross-browser support

and stop there.

---

# Core Goal

Instead of:

Browser

└─ Tabs

└─ Bookmarks

└─ History

└─ Sessions

Lilac introduces:

Records

stored as files.

Every browser becomes merely a frontend.

Firefox

Chrome

Edge

↓

Lilac

↓

Markdown + YAML

---

# Non-goals

No:

- custom renderer
- custom browser engine
- history replacement
- memory optimisation
- isotope tabs
- browser kernel

Those belong to Wisteria.

Lilac should work inside existing browsers.

---

# MVP

### Save current tab

Creates:

---

### Save selected tabs

Creates:

Essentially a OneTab session.

---

### Browse records

Extension popup:

Recent

Collections

Tags

Search

No different from OneTab.

---

### Reopen

Click record.

Lilac:

Collection

↓

URLs

↓

Open tabs

---

# Storage Model

I would make records first-class immediately.

Not:

tab files

but:

record files

---

Page record:

---

type: page

url: https://sea-orm.org

title: SeaORM

---

Collection:

---

type: collection

---

Article:

---

type: article

---

Snippet:

---

type: snippet

---

Even if only page and collection are used initially.

This leaves room for Wisteria later.

---

# File Layout

Something like:

Lilac/

├── Records/

│ ├── page/

│ ├── collection/

│ └── snippet/

│

├── Index/

│ └── records.json

│

└── Assets/

---

# Helper

I would keep it nearly stateless.

API:

save_record

load_record

search_records

delete_record

update_record

Nothing more.

---

# Search

This is where Lilac becomes useful.

Because notes exist.

Example:

---

tags:

- rust

- orm

---

You can search:

tag:rust

inside browser.

Much stronger than OneTab.

---

# Cross Browser Rule

Very important:

A record must never depend on browser-specific features.

Bad:

window_id: 1234

tab_id: 99

Good:

url: https://sea-orm.org

title: SeaORM

Because:

Firefox

Chrome

Edge

must all understand exactly the same file.

---

# Future-proofing for Wisteria

This is the interesting part.

I would already introduce:

id:

type:

created:

modified:

tags:

even when unnecessary.

Because Wisteria ultimately wants:

Everything is a record

and Lilac is the first implementation of that philosophy.

---

So my specification for **Lilac v1** would literally be:

> A cross-browser extension that stores tab collections as Markdown/YAML records on disk through a minimal native helper. Records are human-readable, Git-friendly, Syncthing-friendly, searchable, editable outside the browser, and reopenable from any browser. Browser storage contains only UI preferences; all user data lives in portable files.

That's small enough to build, but it's already the first brick of Orchid → Wisteria rather than a disconnected utility.
```



```
Now to your actual idea:

YAML

[[status]]: [[Reading]]

This is actually valid YAML.

The key is just a string.

YAML doesn't care.

Equivalent to:

YAML

"[[status]]": "[[Reading]]"

or

YAML

status: Reading

The parser simply sees:

Plain Text

key = "[[status]]"

value = "[[Reading]]"

---

So the real question is not:

> Can YAML do this?

but:

> Can Lilac decide this means something?

And the answer is yes.

For example:

YAML

[[status]]: [[Reading]]

[[project]]: [[Orchid]]

[[kind]]: [[Page]]

could be interpreted as:

Plain Text

(status) → (Reading)

(project) → (Orchid)

(kind) → (Page)

which is almost RDF/triple-store style:

Plain Text

Page1 --status--> Reading

Page1 --project--> Orchid

---

What gets really interesting is that then properties themselves become records.

Example:

Markdown

# status

---

type: property

value_type: reference

---

and

Markdown

# Reading

---

type: status

color: blue

---

Then:

YAML

[[status]]: [[Reading]]

is fully self-describing.

No hardcoded enum.

---

I would actually go even further.

Instead of treating YAML as:

Plain Text

key -> value

treat it as:

Plain Text

predicate -> object

meaning:

YAML

[[status]]: [[Reading]]

[[project]]: [[Orchid]]

[[related]]:

- [[Rust]]

- [[ORM]]

is really a graph.

---

The one thing I would _not_ do is require brackets everywhere.

This:

YAML

status: [[Reading]]

project: [[Orchid]]

is probably the pragmatic form.

Because:

- Standard YAML tools still work.
- Obsidian still understands the values.
- Humans can still read it.
- You preserve compatibility.

The schema can simply say:

> Property names are implicitly records.
> 
> `status:` is equivalent to `[[status]]:`

Internally.

Then:

YAML

status: [[Reading]]

becomes:

Plain Text

[[status]] -> [[Reading]]

inside Lilac's graph model.

That gives you most of the benefits without turning every YAML document into wall-to-wall brackets.

In fact, for Orchid/Wisteria I'd seriously consider the rule:

Plain Text

Every property name is a record.

Every linked value is a record.

Every document is a record.

So this:

YAML

status: [[Reading]]

project: [[Orchid]]

would already generate a graph:

Plain Text

(Current Record)

│

├─ status ──► Reading

│

└─ project ─► Orchid

without needing the more verbose:

YAML

[[status]]: [[Reading]]

[[project]]: [[Orchid]]

syntax everywhere. That's probably the sweet spot between graph-native semantics and plain-text usability.
```