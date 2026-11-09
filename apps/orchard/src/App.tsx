import { createEffect, createSignal, For, Index, onCleanup, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap, indentWithTab } from "@codemirror/commands";

interface Block {
  id: string | null;
  kind: any;
  content: string;
  span: [number, number];
  children: Block[];
}

interface Record {
  id: string;
  type: string;
  title: string | null;
  tags: string[];
  status: string | null;
  folder: string | null;
}

interface Settings {
  records_root: string;
  config_root: string;
  registered_folders: string[];
  extension_id: string;
  naming: NamingConfig;
}

interface NamingRule {
  strip: string[];
  replace: [string, string][];
  prefix: string | null;
  suffix: string | null;
  max_len: number | null;
}

interface DomainNaming {
  domain: string;
  rule: NamingRule;
}

interface NamingConfig {
  rules: NamingRule[];
  domains: DomainNaming[];
}

function emptyRule(): NamingRule {
  return { strip: [], replace: [], prefix: null, suffix: null, max_len: null };
}

function emptyNaming(): NamingConfig {
  return { rules: [emptyRule()], domains: [] };
}

function cloneNaming(n: NamingConfig): NamingConfig {
  return JSON.parse(JSON.stringify(n)) as NamingConfig;
}

function splitList(value: string): string[] {
  return value.split(",").map((s) => s.trim()).filter(Boolean);
}

function parsePairs(value: string): [string, string][] {
  return splitList(value).map((pair) => {
    const at = pair.indexOf("=");
    return (at < 0
      ? [pair, ""]
      : [pair.slice(0, at).trim(), pair.slice(at + 1).trim()]) as [string, string];
  });
}

function RuleEditor(props: {
  rule: NamingRule;
  title: string;
  onChange: (rule: NamingRule) => void;
  onRemove?: () => void;
}) {
  const [stripText, setStripText] = createSignal((props.rule.strip || []).join(", "));
  const [replaceText, setReplaceText] = createSignal(
    (props.rule.replace || []).map(([from, to]) => `${from}=${to}`).join(", "),
  );
  const [prefix, setPrefix] = createSignal(props.rule.prefix || "");
  const [suffix, setSuffix] = createSignal(props.rule.suffix || "");
  const [maxLen, setMaxLen] = createSignal(props.rule.max_len?.toString() || "");

  function emit() {
    const max = parseInt(maxLen(), 10);
    props.onChange({
      strip: splitList(stripText()),
      replace: parsePairs(replaceText()),
      prefix: prefix().trim() ? prefix() : null,
      suffix: suffix().trim() ? suffix() : null,
      max_len: Number.isFinite(max) && max > 0 ? max : null,
    });
  }

  return (
    <div class="rule-card">
      <div class="rule-head">
        <b>{props.title}</b>
        <Show when={props.onRemove}>
          <button class="danger" onClick={() => props.onRemove!()}>Remove</button>
        </Show>
      </div>
      <label class="field">
        <span>Strip (comma-separated)</span>
        <input
          value={stripText()}
          placeholder=" - GitHub,  | Hacker News"
          onInput={(e) => setStripText(e.currentTarget.value)}
          onBlur={emit}
        />
      </label>
      <label class="field">
        <span>Replace (from=to, comma-separated)</span>
        <input
          value={replaceText()}
          placeholder="docs.rs=Docs"
          onInput={(e) => setReplaceText(e.currentTarget.value)}
          onBlur={emit}
        />
      </label>
      <div class="row">
        <label class="field">
          <span>Prefix</span>
          <input
            value={prefix()}
            placeholder="[{host}] "
            onInput={(e) => setPrefix(e.currentTarget.value)}
            onBlur={emit}
          />
        </label>
        <label class="field">
          <span>Suffix</span>
          <input
            value={suffix()}
            placeholder=" ({host})"
            onInput={(e) => setSuffix(e.currentTarget.value)}
            onBlur={emit}
          />
        </label>
        <label class="field" style={{ "flex": "0 0 120px" }}>
          <span>Max length</span>
          <input
            type="number"
            min="1"
            value={maxLen()}
            placeholder="120"
            onInput={(e) => setMaxLen(e.currentTarget.value)}
            onBlur={emit}
          />
        </label>
      </div>
      <div class="hint">Templates support <code>{"{host}"}</code> and <code>{"{title}"}</code>.</div>
    </div>
  );
}

interface BlockTypeInfo {
  name: string;
  title: string;
  syntax: string;
  nesting: string;
  rank: number;
  id: string;
  metadata: { name: string; kind: string; required: boolean }[];
}

interface BlockTypeReport {
  types: BlockTypeInfo[];
  valid: boolean;
}

async function fetchRecords(): Promise<Record[]> {
  return (await invoke("list_records")) as Record[];
}

interface MoveReport {
  moved: number;
  errors: string[];
}

async function moveRecords(ids: string[], folder: string): Promise<MoveReport> {
  const json = (await invoke("move_records_cmd", { ids, folder })) as string;
  return JSON.parse(json) as MoveReport;
}

interface RecordMeta {
  id: string;
  type: string;
  title: string | null;
  tags: string[];
  status: string | null;
  url: string | null;
  description: string | null;
  folder: string | null;
}

interface BlockSessionView {
  blocks: Block[];
  body: string;
  record?: RecordMeta;
  fingerprint?: string;
  recovered_edits: number;
  orphaned_wal: string | null;
  external_changed: boolean;
}

async function fetchBlocks(id: string): Promise<BlockSessionView> {
  const json = await invoke("get_blocks", { id }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function updateRecordFields(id: string, fields: { title: string | null; tags: string[]; status: string | null; url: string | null; description: string | null }, expectedFingerprint: string | null): Promise<BlockSessionView> {
  const json = await invoke("update_record_fields_cmd", { id, fields, expectedFingerprint }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function applyBlockEdit(id: string, blockPath: number[], content: string, expectedContent: string): Promise<BlockSessionView> {
  const json = await invoke("apply_block_edit_cmd", { id, blockPath, newContent: content, expectedContent }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function undoBlockEdit(id: string): Promise<BlockSessionView> {
  const json = await invoke("undo_block_edit_cmd", { id }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function redoBlockEdit(id: string): Promise<BlockSessionView> {
  const json = await invoke("redo_block_edit_cmd", { id }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function insertBlock(id: string, afterPath: number[] | null, content: string): Promise<BlockSessionView> {
  const json = await invoke("insert_block_cmd", { id, afterPath, content }) as string;
  return JSON.parse(json) as BlockSessionView;
}

async function deleteBlock(id: string, blockPath: number[], expectedContent: string): Promise<BlockSessionView> {
  const json = await invoke("delete_block_cmd", { id, blockPath, expectedContent }) as string;
  return JSON.parse(json) as BlockSessionView;
}

/// CSS class for a block kind, used to style the editor surface like the
/// Markdown it contains (heading sizes, code, quotes, tables).
function kindClass(kind: any): string {
  if (typeof kind === "string") return "kind-" + kind.toLowerCase();
  if (kind && kind.Heading) return "kind-h" + kind.Heading.level;
  if (kind && kind.Divider) return "kind-divider";
  if (kind && kind.CodeBlock) return "kind-code";
  if (kind && kind.ListItem) return "kind-list";
  if (kind && kind.Custom) return "kind-custom";
  return "kind-block";
}

const ADD_TYPES: { value: string; label: string; prefix: (text: string) => string }[] = [
  { value: "paragraph", label: "Paragraph", prefix: (t) => t },
  { value: "h1", label: "Heading 1", prefix: (t) => "# " + t },
  { value: "h2", label: "Heading 2", prefix: (t) => "## " + t },
  { value: "h3", label: "Heading 3", prefix: (t) => "### " + t },
  { value: "bullet", label: "Bullet", prefix: (t) => "- " + t },
  { value: "quote", label: "Quote", prefix: (t) => "> " + t },
  { value: "code", label: "Code block", prefix: (t) => "```\n" + t + "\n```" },
  { value: "divider", label: "Divider", prefix: () => "---" },
];

/// Structural signature: kinds + child counts. Content edits leave it stable,
/// so the editor is not rebuilt (and focus is not lost) on every blur-save.
function blockShape(blocks: Block[]): string {
  return blocks
    .map((b) => kindLabel(b.kind) + ":" + b.children.length + (b.children.length ? "(" + blockShape(b.children) + ")" : ""))
    .join("|");
}

function kindLabel(kind: any): string {
  if (typeof kind === "string") {
    const names: Record<string, string> = {
      Frontmatter: "frontmatter",
      Paragraph: "paragraph",
      Quote: "quote",
      Callout: "callout",
      Table: "table",
      Math: "math",
      Html: "html",
      Image: "image",
    };
    return names[kind] || kind.toLowerCase();
  }
  if (kind && kind.Heading) return "H" + kind.Heading.level;
  if (kind && kind.Divider) return kind.Divider.rank >= 20 ? "***" : "---";
  if (kind && kind.CodeBlock) return "code" + (kind.CodeBlock.lang ? "(" + kind.CodeBlock.lang + ")" : "");
  if (kind && kind.ListItem) return "list";
  if (kind && kind.Custom) return "custom:" + kind.Custom.name;
  return "block";
}

function kindRank(kind: any): number {
  if (kind && kind.Heading) return 30 + kind.Heading.level - 1;
  if (kind && kind.Divider) return kind.Divider.rank;
  return 255;
}

function isSectionKind(kind: any): boolean {
  return !!(kind && (kind.Heading || kind.Divider));
}

function BlockNode(props: { b: Block; recordId: string; path: number[]; depth: number; indent: boolean; onSaved: (view: BlockSessionView) => void; onError: (message: string) => void; onInsertAfter: (path: number[]) => void; onDelete: (path: number[], content: string) => void }) {
  const section = isSectionKind(props.b.kind);
  return (
    <div class={"block-node " + (section ? "section " : "leaf ") + (props.indent ? "indent-on" : "")}>
      <div class={"block-body " + kindClass(props.b.kind)}>
        <div class="block-label">
          <b>{kindLabel(props.b.kind)}</b>
          <span>rank {kindRank(props.b.kind)}</span>
          <Show when={props.depth > 0}><span>depth {props.depth}</span></Show>
          <span class="block-actions">
            <button class="mini" title="Insert a block after this one" onClick={(e) => { e.stopPropagation(); props.onInsertAfter(props.path); }}>+</button>
            <button class="mini danger" title="Delete this block" onClick={(e) => { e.stopPropagation(); props.onDelete(props.path, props.b.content || ""); }}>×</button>
          </span>
        </div>
        <CodeMirrorBlock
          initial={props.b.content || ""}
          onSave={async (v, expected) => {
            try {
              props.onSaved(await applyBlockEdit(props.recordId, props.path, v, expected));
            } catch (e) {
              props.onError(String(e));
            }
          }}
        />
      </div>
      <Show when={props.b.children.length > 0}>
        <div class="block-children">
          <For each={props.b.children}>
            {(c, i) => (
              <BlockNode
                b={c}
                recordId={props.recordId}
                path={[...props.path, i()]}
                depth={props.depth + 1}
                indent={props.indent}
                onSaved={props.onSaved}
                onError={props.onError}
                onInsertAfter={props.onInsertAfter}
                onDelete={props.onDelete}
              />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

function CodeMirrorBlock(props: { initial: string; onSave: (v: string, expected: string) => void }) {
  let container!: HTMLDivElement;
  let view: EditorView | undefined;
  let lastSaved = props.initial;

  // minimal extensions: view + state + basic keymap, no history/language/autocomplete
  const extensions = [
    keymap.of([...defaultKeymap.filter((k) => !/history/i.test(k.name || "")), indentWithTab]),
    EditorView.lineWrapping,
  ];

  onCleanup(() => {
    view?.destroy();
    view = undefined;
  });

  // mount on mount
  // eslint-disable-next-line solid/reactivity
  setTimeout(() => {
    const state = EditorState.create({ doc: props.initial, extensions });
    view = new EditorView({ state, parent: container });
    view.dom.addEventListener("blur", () => {
      if (!view) return;
      const text = view.state.doc.toString();
      if (text === lastSaved) return;
      const previous = lastSaved;
      lastSaved = text;
      props.onSave(text, previous);
    }, true);
  }, 0);

  return <div ref={container} class="cm-block" />;
}

const ICONS: Record<string, string> = {
  flower: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="24" height="24"><defs><radialGradient id="p1" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#c084fc"/><stop offset="100%" stop-color="#7c3aed"/></radialGradient></defs><circle cx="32" cy="32" r="30" fill="#1e1e2e"/><g fill="url(#p1)" opacity="0.95"><ellipse cx="32" cy="14" rx="9" ry="14"/><ellipse cx="32" cy="14" rx="9" ry="14" transform="rotate(72 32 32)"/><ellipse cx="32" cy="14" rx="9" ry="14" transform="rotate(144 32 32)"/><ellipse cx="32" cy="14" rx="9" ry="14" transform="rotate(216 32 32)"/><ellipse cx="32" cy="14" rx="9" ry="14" transform="rotate(288 32 32)"/></g><circle cx="32" cy="32" r="7" fill="#f5c2e7"/><circle cx="32" cy="32" r="3" fill="#1e1e2e"/></svg>`,
  sideways: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="24" height="24"><defs><linearGradient id="p2" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="#f9a8d4"/><stop offset="100%" stop-color="#be185d"/></linearGradient></defs><circle cx="32" cy="32" r="30" fill="#1e1e2e"/><g fill="url(#p2)" opacity="0.95"><ellipse cx="18" cy="26" rx="10" ry="6" transform="rotate(-30 18 26)"/><ellipse cx="46" cy="26" rx="10" ry="6" transform="rotate(30 46 26)"/><ellipse cx="24" cy="42" rx="10" ry="6" transform="rotate(-60 24 42)"/><ellipse cx="40" cy="42" rx="10" ry="6" transform="rotate(60 40 42)"/><ellipse cx="32" cy="50" rx="9" ry="6"/></g><circle cx="32" cy="26" r="7" fill="#fde68a"/><circle cx="32" cy="26" r="3" fill="#1e1e2e"/></svg>`,
};

export default function App() {
  const [records, setRecords] = createSignal<Record[]>([]);
  const [query, setQuery] = createSignal("");
  const [expanded, setExpanded] = createSignal<string | null>(null);
  const [blocks, setBlocks] = createSignal<Block[]>([]);
  const [blockNotice, setBlockNotice] = createSignal("");
  const [composerOpen, setComposerOpen] = createSignal(false);
  const [composerAfter, setComposerAfter] = createSignal<number[] | null>(null);
  const [composerType, setComposerType] = createSignal("paragraph");
  const [composerText, setComposerText] = createSignal("");
  const [selected, setSelected] = createSignal<Set<string>>(new Set());
  const [moveFolder, setMoveFolder] = createSignal("");
  const [moveNotice, setMoveNotice] = createSignal("");
  const [metaTitle, setMetaTitle] = createSignal("");
  const [metaTags, setMetaTags] = createSignal("");
  const [metaStatus, setMetaStatus] = createSignal("");
  const [metaUrl, setMetaUrl] = createSignal("");
  const [metaDescription, setMetaDescription] = createSignal("");
  const [metaFingerprint, setMetaFingerprint] = createSignal<string | null>(null);
  const [metaNotice, setMetaNotice] = createSignal("");
  const [metaOpen, setMetaOpen] = createSignal(false);
  const [iconChoice, setIconChoice] = createSignal(localStorage.getItem("orchard.icon") || "flower");
  const [indentBlocks, setIndentBlocks] = createSignal(localStorage.getItem("orchard.indentBlocks") !== "false");
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [settings, setSettings] = createSignal<Settings | null>(null);
  const [rootDraft, setRootDraft] = createSignal("");
  const [folderDraft, setFolderDraft] = createSignal("");
  const [extDraft, setExtDraft] = createSignal("");
  const [settingsMsg, setSettingsMsg] = createSignal("");
  const [blockTypes, setBlockTypes] = createSignal<BlockTypeReport | null>(null);
  const [settingsTab, setSettingsTab] = createSignal<"orchid" | "storage" | "lilac" | "naming" | "blocks">("orchid");
  const [namingDraft, setNamingDraft] = createSignal<NamingConfig>(emptyNaming());
  const [previewTitle, setPreviewTitle] = createSignal("Example page - GitHub");
  const [previewUrl, setPreviewUrl] = createSignal("https://github.com/org/repo");
  const [previewResult, setPreviewResult] = createSignal("");

  async function refreshSettings() {
    const s = (await invoke("get_settings")) as Settings;
    setSettings(s);
    setRootDraft(s.records_root);
    setExtDraft(s.extension_id || "");
    setNamingDraft(cloneNaming(s.naming || emptyNaming()));
    const bt = (await invoke("get_block_types")) as BlockTypeReport;
    setBlockTypes(bt);
  }

  async function openSettings() {
    setSettingsMsg("");
    setSettingsOpen(true);
    await refreshSettings();
  }

  async function saveFolders(folders: string[]) {
    await invoke("set_registered_folders_cmd", { folders });
    await refreshSettings();
  }

  async function saveRoot() {
    try {
      const p = (await invoke("set_records_root_cmd", { path: rootDraft() })) as string;
      setSettingsMsg("Records root: " + p);
      await refreshSettings();
      await load();
    } catch (e) {
      setSettingsMsg("Error: " + e);
    }
  }

  async function registerHost() {
    try {
      const r = (await invoke("register_native_host", {
        extensionId: extDraft(),
        hostBinaryPath: null,
      })) as string;
      setSettingsMsg(r);
    } catch (e) {
      setSettingsMsg("Error: " + e);
    }
  }

  async function openFolder(which: "root" | "records" | "config") {
    try {
      const p = (await invoke("open_folder", { which })) as string;
      setSettingsMsg("Opened " + p);
    } catch (e) {
      setSettingsMsg("Error: " + e);
    }
  }

  async function saveNaming() {
    try {
      const saved = (await invoke("set_naming_cmd", { naming: namingDraft() })) as NamingConfig;
      setNamingDraft(cloneNaming(saved));
      setSettingsMsg("Naming rules saved.");
      await refreshSettings();
    } catch (e) {
      setSettingsMsg("Error: " + e);
    }
  }

  async function previewNaming() {
    try {
      const title = (await invoke("preview_naming_cmd", {
        naming: namingDraft(),
        title: previewTitle(),
        url: previewUrl(),
      })) as string;
      setPreviewResult(title);
    } catch (e) {
      setPreviewResult("Error: " + e);
    }
  }

  function updateGlobalRule(index: number, rule: NamingRule) {
    setNamingDraft((d) => ({ ...d, rules: d.rules.map((r, i) => (i === index ? rule : r)) }));
  }
  function removeGlobalRule(index: number) {
    setNamingDraft((d) => ({ ...d, rules: d.rules.filter((_, i) => i !== index) }));
  }
  function addGlobalRule() {
    setNamingDraft((d) => ({ ...d, rules: [...d.rules, emptyRule()] }));
  }
  function updateDomain(index: number, patch: Partial<DomainNaming>) {
    setNamingDraft((d) => ({
      ...d,
      domains: d.domains.map((x, i) => (i === index ? { ...x, ...patch } : x)),
    }));
  }
  function removeDomain(index: number) {
    setNamingDraft((d) => ({ ...d, domains: d.domains.filter((_, i) => i !== index) }));
  }
  function addDomain() {
    setNamingDraft((d) => ({ ...d, domains: [...d.domains, { domain: "", rule: emptyRule() }] }));
  }

  async function load() {
    const q = query().trim();
    const res = q ? await invoke("search", { query: q }) as Record[] : await fetchRecords();
    setRecords(res);
    // Selection is a set of ids; drop ids that no longer exist after a search.
    const visible = new Set(res.map((r) => r.id));
    setSelected((prev) => new Set([...prev].filter((id) => visible.has(id))));
  }

  function toggleSelected(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  const allSelected = () => records().length > 0 && records().every((r) => selected().has(r.id));

  function toggleAll() {
    setSelected(allSelected() ? new Set() : new Set(records().map((r) => r.id)));
  }

  function startMove(id: string, folder: string | null) {
    setSelected(new Set([id]));
    setMoveFolder(folder || "");
    setMoveNotice("");
    setTimeout(() => document.getElementById("move-folder")?.focus(), 0);
  }

  async function moveSelected() {
    const ids = [...selected()];
    if (ids.length === 0) return;
    try {
      const report = await moveRecords(ids, moveFolder());
      const failed = report.errors.length;
      setMoveNotice(
        `Moved ${report.moved} record${report.moved === 1 ? "" : "s"}` +
        (failed ? `; ${failed} failed: ${report.errors.join("; ")}` : ""),
      );
      setSelected(new Set());
      const openId = expanded();
      await load();
      // A moved record's file changed location; refresh its view so the
      // metadata panel gets the new version fingerprint.
      if (openId && ids.includes(openId)) {
        await reloadBlocks(openId);
      }
    } catch (e) {
      setMoveNotice("Move failed: " + String(e));
    }
  }

  async function toggle(id: string) {
    setBlockNotice("");
    setComposerOpen(false);
    setMetaNotice("");
    setMetaOpen(false);
    if (expanded() === id) { setExpanded(null); setBlocks([]); return; }
    setExpanded(id);
    try {
      const view = await fetchBlocks(id);
      setBlocks(view.blocks);
      applyMeta(view);
      if (view.recovered_edits) {
        setBlockNotice(`Recovered ${view.recovered_edits} unsaved edit${view.recovered_edits === 1 ? "" : "s"} from the write-ahead log.`);
      } else if (view.orphaned_wal) {
        setBlockNotice(`Ignored an out-of-date write-ahead log, kept aside at ${view.orphaned_wal}`);
      } else if (view.external_changed) {
        setBlockNotice("This file changed outside Orchard; the view was refreshed.");
      }
    } catch (e) {
      setBlockNotice("Could not open record: " + String(e));
    }
  }

  function handleSaved(id: string, view: BlockSessionView) {
    setBlockNotice("");
    if (view.external_changed) {
      setBlockNotice("Incorporated changes another program made to this file.");
    }
    syncFingerprint(view);
    // Only rebuild the tree when the document shape actually changed.
    if (blockShape(view.blocks) !== blockShape(blocks())) {
      setBlocks(view.blocks);
    }
  }

  function syncFingerprint(view: BlockSessionView) {
    if (view.fingerprint) setMetaFingerprint(view.fingerprint);
  }

  /// Load the YAML fields into the metadata form. Called when a record opens
  /// and after a metadata save; ordinary block saves only refresh the version.
  function applyMeta(view: BlockSessionView) {
    syncFingerprint(view);
    const record = view.record;
    if (!record) return;
    setMetaTitle(record.title || "");
    setMetaTags((record.tags || []).join(", "));
    setMetaStatus(record.status || "");
    setMetaUrl(record.url || "");
    setMetaDescription(record.description || "");
  }

  async function saveMetadata(id: string) {
    try {
      const view = await updateRecordFields(id, {
        title: metaTitle().trim() || null,
        tags: metaTags().split(",").map((t) => t.trim()).filter(Boolean),
        status: metaStatus().trim() || null,
        url: metaUrl().trim() || null,
        description: metaDescription().trim() || null,
      }, metaFingerprint());
      applyMeta(view);
      setBlocks(view.blocks);
      setMetaNotice("Saved metadata.");
      const saved = view.record;
      if (saved) {
        // Update the card in place; a full reload would rebuild the list and
        // collapse the metadata panel the user is working in.
        setRecords((prev) => prev.map((r) => r.id === id
          ? { ...r, title: saved.title, tags: saved.tags, status: saved.status }
          : r));
      }
    } catch (e) {
      const message = String(e);
      setMetaNotice("Save failed: " + message);
      if (message.includes("changed since")) {
        await reloadBlocks(id);
      }
    }
  }

  async function undo(id: string) {
    try {
      const view = await undoBlockEdit(id);
      setBlocks(view.blocks);
      syncFingerprint(view);
      setBlockNotice("Undid the last edit.");
    } catch (e) {
      setBlockNotice("Undo failed: " + String(e));
    }
  }

  async function redo(id: string) {
    try {
      const view = await redoBlockEdit(id);
      setBlocks(view.blocks);
      syncFingerprint(view);
      setBlockNotice("Redid the undone edit.");
    } catch (e) {
      setBlockNotice("Redo failed: " + String(e));
    }
  }

  async function reloadBlocks(id: string) {
    try {
      const view = await fetchBlocks(id);
      setBlocks(view.blocks);
      applyMeta(view);
    } catch {
      /* the notice already explains the failure */
    }
  }

  function handleSaveError(id: string, message: string) {
    setBlockNotice("Save failed: " + message);
    // The block path may now point at a different block. Reload so the next
    // edit is addressed correctly and the user sees the current text.
    if (message.includes("changed on disk")) {
      reloadBlocks(id);
    }
  }

  function openComposer(after: number[] | null) {
    setComposerAfter(after);
    setComposerText("");
    setComposerType("paragraph");
    setComposerOpen(true);
  }

  async function addBlock(id: string) {
    const spec = ADD_TYPES.find((t) => t.value === composerType()) || ADD_TYPES[0];
    const text = composerText().trim();
    if (spec.value !== "divider" && !text) {
      setBlockNotice("Type something to add, or choose Divider.");
      return;
    }
    try {
      const view = await insertBlock(id, composerAfter(), spec.prefix(text));
      setBlocks(view.blocks);
      syncFingerprint(view);
      setComposerOpen(false);
      setComposerText("");
      setBlockNotice("Added a block.");
    } catch (e) {
      setBlockNotice("Add failed: " + String(e));
    }
  }

  async function removeBlock(id: string, path: number[], content: string) {
    if (!confirm("Delete this block and its children?")) return;
    try {
      const view = await deleteBlock(id, path, content);
      setBlocks(view.blocks);
      syncFingerprint(view);
      setComposerOpen(false);
      setBlockNotice("Deleted a block.");
    } catch (e) {
      const message = String(e);
      setBlockNotice("Delete failed: " + message);
      if (message.includes("changed on disk")) {
        reloadBlocks(id);
      }
    }
  }

  // initial load
  load();

  // persist + apply icon choice
  createEffect(() => {
    const c = iconChoice();
    localStorage.setItem("orchard.icon", c);
    const link = document.querySelector('link[rel="icon"]') as HTMLLinkElement | null;
    if (link) link.href = c === "sideways" ? "/icon2.svg" : "/icon.svg";
  });

  createEffect(() => {
    localStorage.setItem("orchard.indentBlocks", String(indentBlocks()));
  });

  return (
    <>
      <header>
        <span innerHTML={ICONS[iconChoice()]} style={{ "margin-right": "4px" }} />
        <h1>Orchard</h1>
        <button onClick={() => openSettings()} title="Settings">⚙ Settings</button>
        <input
          type="text"
          placeholder="search (tag:rust, type:page, ...)"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") load(); }}
        />
        <button onClick={() => load()}>Search</button>
      </header>
      <Show when={settingsOpen()}>
        <div class="settings-modal" onClick={(e) => { if (e.target === e.currentTarget) setSettingsOpen(false); }}>
          <div class="settings-panel" onClick={(e) => e.stopPropagation()}>
            <div class="settings-head">
              <h2>Settings</h2>
              <button onClick={() => setSettingsOpen(false)}>Close</button>
            </div>

            <div class="settings-tabs">
              <button class={"tab " + (settingsTab() === "orchid" ? "active" : "")} onClick={() => setSettingsTab("orchid")}>Orchid</button>
              <button class={"tab " + (settingsTab() === "storage" ? "active" : "")} onClick={() => setSettingsTab("storage")}>Storage</button>
              <button class={"tab " + (settingsTab() === "lilac" ? "active" : "")} onClick={() => setSettingsTab("lilac")}>Lilac</button>
              <button class={"tab " + (settingsTab() === "naming" ? "active" : "")} onClick={() => setSettingsTab("naming")}>Naming</button>
              <button class={"tab " + (settingsTab() === "blocks" ? "active" : "")} onClick={() => setSettingsTab("blocks")}>Block types</button>
            </div>

            <div class="settings-body">
              <Show when={settingsTab() === "orchid"}>
                <section class="settings-section">
                  <h3>Appearance</h3>
                  <label class="row">
                    <span>App icon</span>
                    <select value={iconChoice()} onChange={(e) => setIconChoice(e.currentTarget.value)}>
                      <option value="flower">Flower (top view)</option>
                      <option value="sideways">Flower (side view)</option>
                    </select>
                  </label>
                  <div class="hint">The icon is also embedded in <code class="mono">orchard.exe</code>; rebuild to update the taskbar and window icon.</div>
                  <label class="row">
                    <input
                      type="checkbox"
                      checked={indentBlocks()}
                      onChange={(e) => setIndentBlocks(e.currentTarget.checked)}
                    />
                    <span>Indent block hierarchy</span>
                  </label>
                  <div class="hint">Off keeps every block at the same left edge; hierarchy stays internal and is shown by rank/depth labels.</div>
                </section>

                <section class="settings-section">
                  <h3>Orchid configuration</h3>
                  <div class="row">
                    <code class="mono" style={{ flex: "1", overflow: "auto" }}>{settings()?.config_root || "…"}</code>
                    <button onClick={() => openFolder("config")}>Open config folder</button>
                  </div>
                  <div class="hint">Orchid settings (window, icon, app folders) live here. Lilac's browser bridge settings are on their own tab.</div>
                </section>

                <section class="settings-section">
                  <h3>About</h3>
                  <div class="hint">Orchard 0.1.0 · single executable: GUI + native-messaging host.</div>
                </section>
              </Show>

              <Show when={settingsTab() === "storage"}>
                <section class="settings-section">
                  <h3>Records root</h3>
                  <div class="row">
                    <input value={rootDraft()} onInput={(e) => setRootDraft(e.currentTarget.value)} />
                    <button class="primary" onClick={() => saveRoot()}>Save root</button>
                    <button onClick={() => openFolder("root")}>Open</button>
                  </div>
                  <div class="hint">Portable Markdown + YAML records. Indexes are caches and can be rebuilt from this folder.</div>
                </section>

                <section class="settings-section">
                  <h3>Folders</h3>
                  <div class="row" style={{ "justify-content": "space-between" }}>
                    <span class="hint">Destinations Lilac can save into, relative to <code>Records/</code>.</span>
                    <button onClick={() => openFolder("records")}>Open Records</button>
                  </div>
                  <div style={{ display: "grid", gap: "6px" }}>
                    <For each={settings()?.registered_folders || []}>
                      {(f) => (
                        <div class="list-row">
                          <code class="mono">Records/{f}</code>
                          <button class="danger" onClick={() => saveFolders((settings()?.registered_folders || []).filter((x) => x !== f))}>Remove</button>
                        </div>
                      )}
                    </For>
                    <Show when={!(settings()?.registered_folders || []).length}>
                      <div class="hint">No registered folders. Lilac falls back to <code>Records/&lt;type&gt;</code>.</div>
                    </Show>
                    <div class="row">
                      <input value={folderDraft()} placeholder="e.g. Reading/Articles" onInput={(e) => setFolderDraft(e.currentTarget.value)} />
                      <button onClick={async () => {
                        const f = folderDraft().trim().replace(/^\/+|\/+$/g, "");
                        if (!f) return;
                        const cur = settings()?.registered_folders || [];
                        if (!cur.includes(f)) await saveFolders([...cur, f]);
                        setFolderDraft("");
                      }}>Add folder</button>
                    </div>
                  </div>
                </section>

                <section class="settings-section">
                  <h3>Config root</h3>
                  <div class="row">
                    <code class="mono" style={{ flex: "1", overflow: "auto" }}>{settings()?.config_root || "…"}</code>
                    <button onClick={() => openFolder("config")}>Open</button>
                  </div>
                  <div class="hint">Machine-specific settings, logs, rules, WAL and caches. Regenerable; never the source of truth.</div>
                </section>
              </Show>

              <Show when={settingsTab() === "lilac"}>
                <section class="settings-section">
                  <h3>Browser bridge</h3>
                  <div class="hint">
                    Lilac is the browser extension. Orchard is the store. This executable is also the native-messaging host, so the browser launches this same <code>orchard.exe</code>.
                  </div>
                  <div class="row">
                    <input value={extDraft()} placeholder="Chrome extension ID" onInput={(e) => setExtDraft(e.currentTarget.value)} />
                    <button class="primary" onClick={() => registerHost()}>Register native host</button>
                  </div>
                  <div class="hint">Re-register whenever the extension ID or the executable path changes.</div>
                </section>

                <section class="settings-section">
                  <h3>Where Lilac saves</h3>
                  <div class="row">
                    <code class="mono" style={{ flex: "1", overflow: "auto" }}>{settings()?.records_root || "…"}</code>
                    <button onClick={() => openFolder("root")}>Open</button>
                  </div>
                  <div class="hint">Lilac may only choose from the registered folders in the Storage tab.</div>
                </section>

                <section class="settings-section">
                  <h3>Host manifest</h3>
                  <div class="hint mono">%LOCALAPPDATA%\Lilac\lilac_host.json</div>
                  <div class="hint">Writes the Chrome and Firefox native-messaging keys. Restart the browser after registering.</div>
                </section>
              </Show>

              <Show when={settingsTab() === "naming"}>
                <section class="settings-section">
                  <h3>Global rules</h3>
                  <div class="hint">Applied in order when no domain override matches. A later rule sees the previous rule's result.</div>
                  <Index each={namingDraft().rules}>
                    {(rule, i) => (
                      <RuleEditor
                        rule={rule()}
                        title={`Global rule ${i + 1}`}
                        onChange={(r) => updateGlobalRule(i, r)}
                        onRemove={() => removeGlobalRule(i)}
                      />
                    )}
                  </Index>
                  <div class="row">
                    <button onClick={addGlobalRule}>Add global rule</button>
                  </div>
                </section>

                <section class="settings-section">
                  <h3>Domain overrides</h3>
                  <div class="hint">
                    Match a host exactly (<code class="mono">github.com</code>) or a subdomain tree
                    (<code class="mono">*.example.com</code>). The first match wins and replaces the
                    global rules for that host; <code class="mono">www.</code> is ignored.
                  </div>
                  <Index each={namingDraft().domains}>
                    {(domain, i) => (
                      <div class="rule-card">
                        <div class="row">
                          <input
                            placeholder="example.com or *.example.com"
                            value={domain().domain}
                            onInput={(e) => updateDomain(i, { domain: e.currentTarget.value })}
                          />
                          <button class="danger" onClick={() => removeDomain(i)}>Remove</button>
                        </div>
                        <RuleEditor
                          rule={domain().rule}
                          title="Override rule"
                          onChange={(r) => updateDomain(i, { rule: r })}
                        />
                      </div>
                    )}
                  </Index>
                  <div class="row">
                    <button onClick={addDomain}>Add domain override</button>
                  </div>
                </section>

                <section class="settings-section">
                  <h3>Preview</h3>
                  <div class="row">
                    <input
                      value={previewTitle()}
                      placeholder="Page title"
                      onInput={(e) => setPreviewTitle(e.currentTarget.value)}
                    />
                    <input
                      value={previewUrl()}
                      placeholder="https://example.com/page"
                      onInput={(e) => setPreviewUrl(e.currentTarget.value)}
                    />
                    <button onClick={previewNaming}>Preview</button>
                  </div>
                  <Show when={previewResult()}>
                    <div class="hint">Result: <b>{previewResult()}</b></div>
                  </Show>
                </section>

                <div class="row">
                  <button class="primary" onClick={saveNaming}>Save naming rules</button>
                  <span class="hint">Used by the browser bridge when it saves a tab.</span>
                </div>
              </Show>

              <Show when={settingsTab() === "blocks"}>
                <section class="settings-section">
                  <h3>Declared block types</h3>
                  <div class={"hint " + (blockTypes()?.valid ? "status-ok" : "status-bad")}>
                    {blockTypes() ? `${blockTypes()!.types.length} registered · registry ${blockTypes()!.valid ? "valid" : "INVALID"}` : "…"}
                  </div>
                  <div class="registry">
                    <For each={blockTypes()?.types || []}>
                      {(t) => (
                        <div
                          class="registry-row"
                          title={t.metadata.length ? "metadata: " + t.metadata.map((m) => m.name + ":" + m.kind + (m.required ? "!" : "")).join(", ") : "no metadata fields"}
                        >
                          <span class="name">{t.name}</span>
                          <span class="rank">r{t.rank}</span>
                          <span class="nesting">{t.nesting}</span>
                          <span class="id">{t.id}</span>
                        </div>
                      )}
                    </For>
                  </div>
                  <div class="hint">The parser and tree builder read this registry; adding a type is one declaration. Hover a row for metadata fields.</div>
                </section>
              </Show>
            </div>

            <Show when={settingsMsg()}>
              <div class="hint" style={{ "word-break": "break-word" }}>{settingsMsg()}</div>
            </Show>
          </div>
        </div>
      </Show>
      <Show when={records().length > 0}>
        <div class="list-tools">
          <label class="select-all">
            <input type="checkbox" checked={allSelected()} onChange={toggleAll} />
            Select all
          </label>
          <Show when={selected().size > 0}>
            <span class="hint">{selected().size} selected</span>
            <input
              id="move-folder"
              class="move-folder"
              list="folder-choices"
              placeholder="folder under Records/ (blank = type folder)"
              value={moveFolder()}
              onInput={(e) => setMoveFolder(e.currentTarget.value)}
              onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); moveSelected(); } }}
            />
            <datalist id="folder-choices">
              <For each={settings()?.registered_folders || ["page", "collection", "bookmark", "snapshot", "note", "snippet"]}>
                {(f) => <option value={f} />}
              </For>
            </datalist>
            <button class="primary" onClick={moveSelected}>Move</button>
            <button onClick={() => { setSelected(new Set()); setMoveNotice(""); }}>Clear</button>
          </Show>
          <Show when={moveNotice()}>
            <span class={moveNotice().startsWith("Moved") && !moveNotice().includes("failed") ? "hint status-ok" : "error"}>{moveNotice()}</span>
          </Show>
        </div>
      </Show>
      <div style={{ display: "grid", gap: "8px", "max-height": "80vh", "overflow": "auto", "margin-top": "12px" }}>
        <For each={records()}>
          {(r) => (
            <div class="card" onClick={() => toggle(r.id)} style={{ cursor: "pointer" }}>
              <div class="card-head">
                <input
                  type="checkbox"
                  class="select-box"
                  checked={selected().has(r.id)}
                  title="Select for moving"
                  onClick={(e) => e.stopPropagation()}
                  onChange={() => toggleSelected(r.id)}
                />
                <div class="title">{r.title || r.id}</div>
                <Show when={r.folder}>
                  <span class="folder-badge" title={"Records/" + r.folder}>Records/{r.folder}</span>
                </Show>
              </div>
              <div class="meta">{r.id} · {r.type}{r.status ? " · " + r.status : ""}</div>
              <div class="tags">{(r.tags || []).join(", ")}</div>
              <Show when={expanded() === r.id}>
                <div class="blocks" style={{ "margin-top": "8px", cursor: "default" }} onClick={(e) => e.stopPropagation()}>
                  <div class="row" style={{ "margin-bottom": "6px" }}>
                    <button onClick={(e) => { e.stopPropagation(); undo(r.id); }} title="Undo the last block edit">Undo</button>
                    <button onClick={(e) => { e.stopPropagation(); redo(r.id); }} title="Redo the last undone block edit">Redo</button>
                    <button onClick={(e) => { e.stopPropagation(); openComposer(null); }} title="Add a block at the end of the body">Add block</button>
                    <button onClick={(e) => { e.stopPropagation(); startMove(r.id, r.folder); }} title="Move this record to another folder under Records/">Move…</button>
                    <Show when={blockNotice()}>
                      <span class="error">{blockNotice()}</span>
                    </Show>
                  </div>
                  <details class="meta-editor" open={metaOpen()} onToggle={(e) => setMetaOpen(e.currentTarget.open)}>
                    <summary>Metadata</summary>
                    <div class="meta-grid">
                      <label class="field">
                        <span>Title</span>
                        <input value={metaTitle()} onInput={(e) => setMetaTitle(e.currentTarget.value)} />
                      </label>
                      <label class="field">
                        <span>Status</span>
                        <input value={metaStatus()} placeholder="e.g. Reading" onInput={(e) => setMetaStatus(e.currentTarget.value)} />
                      </label>
                      <label class="field">
                        <span>URL</span>
                        <input value={metaUrl()} placeholder="https://…" onInput={(e) => setMetaUrl(e.currentTarget.value)} />
                      </label>
                      <label class="field">
                        <span>Tags (comma separated)</span>
                        <input value={metaTags()} placeholder="rust, local-first" onInput={(e) => setMetaTags(e.currentTarget.value)} />
                      </label>
                    </div>
                    <label class="field">
                      <span>Description</span>
                      <input value={metaDescription()} onInput={(e) => setMetaDescription(e.currentTarget.value)} />
                    </label>
                    <div class="row" style={{ "margin-top": "8px" }}>
                      <button class="primary" onClick={(e) => { e.stopPropagation(); saveMetadata(r.id); }}>Save metadata</button>
                      <span class="hint">Writes YAML frontmatter only; the body stays as it is.</span>
                      <Show when={metaNotice()}>
                        <span class={metaNotice().startsWith("Saved") ? "hint status-ok" : "error"}>{metaNotice()}</span>
                      </Show>
                    </div>
                  </details>
                  <Show when={composerOpen()}>
                    <div class="composer" onClick={(e) => e.stopPropagation()}>
                      <select value={composerType()} onChange={(e) => setComposerType(e.currentTarget.value)}>
                        <For each={ADD_TYPES}>{(t) => <option value={t.value}>{t.label}</option>}</For>
                      </select>
                      <input
                        autofocus
                        value={composerText()}
                        disabled={composerType() === "divider"}
                        placeholder={composerType() === "divider" ? "Divider needs no text" : composerAfter() ? "Text to insert after the selected block…" : "Text to add at the end…"}
                        onInput={(e) => setComposerText(e.currentTarget.value)}
                        onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addBlock(r.id); } }}
                      />
                      <button class="primary" onClick={() => addBlock(r.id)}>Add</button>
                      <button onClick={() => setComposerOpen(false)}>Cancel</button>
                      <span class="hint">{composerAfter() ? "after position " + JSON.stringify(composerAfter()) : "at the end"}</span>
                    </div>
                  </Show>
                  <Show when={blocks().length === 0}>
                    <div class="hint">This record has no body yet. Use <b>Add block</b> to start writing.</div>
                  </Show>
                  <For each={blocks()}>
                    {(b, i) => (
                      <BlockNode
                        b={b}
                        recordId={r.id}
                        path={[i()]}
                        depth={0}
                        indent={indentBlocks()}
                        onSaved={(view) => handleSaved(r.id, view)}
                        onError={(message) => handleSaveError(r.id, message)}
                        onInsertAfter={(path) => openComposer(path)}
                        onDelete={(path, content) => removeBlock(r.id, path, content)}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </div>
          )}
        </For>
      </div>
      <footer style={{ "margin-top": "8px", "font-size": "12px", color: "#a6adc8" }}>
        {records().length} records
      </footer>
    </>
  );
}
