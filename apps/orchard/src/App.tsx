import { createEffect, createSignal, For, Show } from "solid-js";
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
}

interface Settings {
  records_root: string;
  config_root: string;
  registered_folders: string[];
  extension_id: string;
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

interface BlockSessionView {
  blocks: Block[];
  body: string;
  recovered_edits: number;
  orphaned_wal: string | null;
  external_changed: boolean;
}

async function fetchBlocks(id: string): Promise<BlockSessionView> {
  const json = await invoke("get_blocks", { id }) as string;
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

function BlockNode(props: { b: Block; recordId: string; path: number[]; depth: number; indent: boolean; onSaved: (view: BlockSessionView) => void; onError: (message: string) => void }) {
  const section = isSectionKind(props.b.kind);
  return (
    <div class={"block-node " + (section ? "section " : "leaf ") + (props.indent ? "indent-on" : "")}>
      <div class="block-body">
        <div class="block-label">
          <b>{kindLabel(props.b.kind)}</b>
          <span>rank {kindRank(props.b.kind)}</span>
          <Show when={props.depth > 0}><span>depth {props.depth}</span></Show>
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

  return <div ref={container} style={{ "min-height": "40px", background: "#313244", "border-radius": "4px", padding: "4px" }} />;
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
  const [iconChoice, setIconChoice] = createSignal(localStorage.getItem("orchard.icon") || "flower");
  const [indentBlocks, setIndentBlocks] = createSignal(localStorage.getItem("orchard.indentBlocks") !== "false");
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [settings, setSettings] = createSignal<Settings | null>(null);
  const [rootDraft, setRootDraft] = createSignal("");
  const [folderDraft, setFolderDraft] = createSignal("");
  const [extDraft, setExtDraft] = createSignal("");
  const [settingsMsg, setSettingsMsg] = createSignal("");
  const [blockTypes, setBlockTypes] = createSignal<BlockTypeReport | null>(null);
  const [settingsTab, setSettingsTab] = createSignal<"orchid" | "storage" | "lilac" | "blocks">("orchid");

  async function refreshSettings() {
    const s = (await invoke("get_settings")) as Settings;
    setSettings(s);
    setRootDraft(s.records_root);
    setExtDraft(s.extension_id || "");
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

  async function load() {
    const q = query().trim();
    const res = q ? await invoke("search", { query: q }) as Record[] : await fetchRecords();
    setRecords(res);
  }

  async function toggle(id: string) {
    setBlockNotice("");
    if (expanded() === id) { setExpanded(null); setBlocks([]); return; }
    setExpanded(id);
    try {
      const view = await fetchBlocks(id);
      setBlocks(view.blocks);
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
    // Only rebuild the tree when the document shape actually changed.
    if (blockShape(view.blocks) !== blockShape(blocks())) {
      setBlocks(view.blocks);
    }
  }

  async function undo(id: string) {
    try {
      const view = await undoBlockEdit(id);
      setBlocks(view.blocks);
      setBlockNotice("Undid the last edit.");
    } catch (e) {
      setBlockNotice("Undo failed: " + String(e));
    }
  }

  async function reloadBlocks(id: string) {
    try {
      const view = await fetchBlocks(id);
      setBlocks(view.blocks);
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
      <div style={{ display: "grid", gap: "8px", "max-height": "80vh", "overflow": "auto", "margin-top": "12px" }}>
        <For each={records()}>
          {(r) => (
            <div class="card" onClick={() => toggle(r.id)} style={{ cursor: "pointer" }}>
              <div class="title">{r.title || r.id}</div>
              <div class="meta">{r.id} · {r.type}{r.status ? " · " + r.status : ""}</div>
              <div class="tags">{(r.tags || []).join(", ")}</div>
              <Show when={expanded() === r.id}>
                <div class="blocks" style={{ "margin-top": "8px" }}>
                  <div class="row" style={{ "margin-bottom": "6px" }}>
                    <button onClick={(e) => { e.stopPropagation(); undo(r.id); }} title="Undo the last block edit">Undo</button>
                    <Show when={blockNotice()}>
                      <span class="error">{blockNotice()}</span>
                    </Show>
                  </div>
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
