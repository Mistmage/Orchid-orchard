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

async function fetchBlocks(id: string): Promise<Block[]> {
  const json = await invoke("get_blocks", { id }) as string;
  return JSON.parse(json) as Block[];
}

async function applyBlockEdit(id: string, blockId: string, content: string) {
  await invoke("apply_block_edit_cmd", { id, blockId, newContent: content });
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

function BlockNode(props: { b: Block; recordId: string; depth: number }) {
  return (
    <div style={{ "margin-left": props.depth ? "14px" : "0", "margin-bottom": "6px" }}>
      <div style={{ "font-size": "11px", color: "#a6adc8", "margin-bottom": "2px" }}>
        <b>{kindLabel(props.b.kind)}</b>
        <span style={{ "margin-left": "6px", opacity: 0.7 }}>rank {kindRank(props.b.kind)}</span>
      </div>
      <CodeMirrorBlock
        initial={props.b.content || ""}
        onSave={(v) => applyBlockEdit(props.recordId, props.b.id!, v).catch(() => {})}
      />
      <For each={props.b.children}>
        {(c) => <BlockNode b={c} recordId={props.recordId} depth={props.depth + 1} />}
      </For>
    </div>
  );
}

function CodeMirrorBlock(props: { initial: string; onSave: (v: string) => void }) {
  let container!: HTMLDivElement;
  let view: EditorView | undefined;

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
      if (view) props.onSave(view.state.doc.toString());
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
  const [iconChoice, setIconChoice] = createSignal(localStorage.getItem("orchard.icon") || "flower");
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [settings, setSettings] = createSignal<Settings | null>(null);
  const [rootDraft, setRootDraft] = createSignal("");
  const [folderDraft, setFolderDraft] = createSignal("");
  const [extDraft, setExtDraft] = createSignal("");
  const [settingsMsg, setSettingsMsg] = createSignal("");
  const [blockTypes, setBlockTypes] = createSignal<BlockTypeReport | null>(null);

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

  async function load() {
    const q = query().trim();
    const res = q ? await invoke("search", { query: q }) as Record[] : await fetchRecords();
    setRecords(res);
  }

  async function toggle(id: string) {
    if (expanded() === id) { setExpanded(null); setBlocks([]); return; }
    setExpanded(id);
    setBlocks(await fetchBlocks(id));
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
        <div
          style={{
            position: "fixed", inset: "0", background: "rgba(0,0,0,0.55)",
            display: "flex", "align-items": "flex-start", "justify-content": "center",
            "z-index": "100", padding: "48px 16px", overflow: "auto",
          }}
          onClick={(e) => { if (e.target === e.currentTarget) setSettingsOpen(false); }}
        >
          <div style={{
            width: "min(640px, 100%)", background: "#1e1e2e", border: "1px solid #45475a",
            "border-radius": "12px", padding: "20px", display: "grid", gap: "16px",
          }}>
            <div style={{ display: "flex", "align-items": "center", "justify-content": "space-between" }}>
              <h2 style={{ margin: "0", "font-size": "18px" }}>Settings</h2>
              <button onClick={() => setSettingsOpen(false)}>Close</button>
            </div>

            <section>
              <h3 style={{ "font-size": "13px", "text-transform": "uppercase", color: "#a6adc8", margin: "0 0 6px" }}>Appearance</h3>
              <label style={{ display: "flex", gap: "8px", "align-items": "center" }}>
                <span>Icon</span>
                <select
                  value={iconChoice()}
                  onChange={(e) => setIconChoice(e.currentTarget.value)}
                  style={{ padding: "4px", "border-radius": "6px", background: "#313244", color: "#cdd6f4", border: "1px solid #45475a" }}
                >
                  <option value="flower">Flower (top view)</option>
                  <option value="sideways">Flower (side view)</option>
                </select>
              </label>
            </section>

            <section>
              <h3 style={{ "font-size": "13px", "text-transform": "uppercase", color: "#a6adc8", margin: "0 0 6px" }}>Storage</h3>
              <div style={{ display: "flex", gap: "8px", "align-items": "center", "flex-wrap": "wrap" }}>
                <input
                  value={rootDraft()}
                  onInput={(e) => setRootDraft(e.currentTarget.value)}
                  style={{ flex: "1 1 280px", padding: "6px 10px", "border-radius": "6px", border: "1px solid #45475a", background: "#313244", color: "#cdd6f4" }}
                />
                <button onClick={() => saveRoot()}>Save root</button>
              </div>
              <div style={{ "font-size": "12px", color: "#a6adc8", "margin-top": "4px" }}>
                Config: {settings()?.config_root || "…"}
              </div>
            </section>

            <section>
              <h3 style={{ "font-size": "13px", "text-transform": "uppercase", color: "#a6adc8", margin: "0 0 6px" }}>Registered folders</h3>
              <div style={{ display: "grid", gap: "6px" }}>
                <For each={settings()?.registered_folders || []}>
                  {(f) => (
                    <div style={{ display: "flex", gap: "8px", "align-items": "center" }}>
                      <code style={{ flex: "1", background: "#313244", padding: "4px 8px", "border-radius": "4px" }}>{f}</code>
                      <button onClick={() => saveFolders((settings()?.registered_folders || []).filter((x) => x !== f))}>Remove</button>
                    </div>
                  )}
                </For>
                <Show when={!(settings()?.registered_folders || []).length}>
                  <div style={{ "font-size": "12px", color: "#a6adc8" }}>
                    None yet. Lilac saves to <code>Records/&lt;type&gt;</code> until you add folders.
                  </div>
                </Show>
                <div style={{ display: "flex", gap: "8px" }}>
                  <input
                    value={folderDraft()}
                    placeholder="e.g. Reading/Articles"
                    onInput={(e) => setFolderDraft(e.currentTarget.value)}
                    style={{ flex: "1", padding: "6px 10px", "border-radius": "6px", border: "1px solid #45475a", background: "#313244", color: "#cdd6f4" }}
                  />
                  <button onClick={async () => {
                    const f = folderDraft().trim();
                    if (!f) return;
                    const cur = settings()?.registered_folders || [];
                    if (!cur.includes(f)) await saveFolders([...cur, f]);
                    setFolderDraft("");
                  }}>Add</button>
                </div>
              </div>
            </section>

            <section>
              <h3 style={{ "font-size": "13px", "text-transform": "uppercase", color: "#a6adc8", margin: "0 0 6px" }}>Browser bridge (Lilac)</h3>
              <div style={{ display: "flex", gap: "8px", "flex-wrap": "wrap" }}>
                <input
                  value={extDraft()}
                  placeholder="Chrome extension ID"
                  onInput={(e) => setExtDraft(e.currentTarget.value)}
                  style={{ flex: "1 1 280px", padding: "6px 10px", "border-radius": "6px", border: "1px solid #45475a", background: "#313244", color: "#cdd6f4" }}
                />
                <button onClick={() => registerHost()}>Register native host</button>
              </div>
              <div style={{ "font-size": "12px", color: "#a6adc8", "margin-top": "4px" }}>
                Registers this same <code>orchard.exe</code> for Chrome and Firefox.
              </div>
            </section>

            <section>
              <h3 style={{ "font-size": "13px", "text-transform": "uppercase", color: "#a6adc8", margin: "0 0 6px" }}>Block types</h3>
              <div style={{ "font-size": "12px", "margin-bottom": "6px", color: blockTypes()?.valid ? "#a6e3a1" : "#f38ba8" }}>
                {blockTypes() ? `${blockTypes()!.types.length} registered · registry ${blockTypes()!.valid ? "valid" : "INVALID"}` : "…"}
              </div>
              <div style={{ display: "grid", gap: "3px", "max-height": "180px", overflow: "auto" }}>
                <For each={blockTypes()?.types || []}>
                  {(t) => (
                    <div
                      title={t.metadata.length ? "metadata: " + t.metadata.map((m) => m.name + ":" + m.kind + (m.required ? "!" : "")).join(", ") : "no metadata fields"}
                      style={{ display: "flex", gap: "8px", "font-size": "12px", "font-family": "monospace" }}
                    >
                      <span style={{ width: "110px" }}>{t.name}</span>
                      <span style={{ width: "60px", opacity: 0.8 }}>r{t.rank}</span>
                      <span style={{ flex: "1", opacity: 0.7 }}>{t.nesting}</span>
                      <span style={{ opacity: 0.55 }}>{t.id}</span>
                    </div>
                  )}
                </For>
              </div>
            </section>

            <Show when={settingsMsg()}>
              <div style={{ "font-size": "12px", color: "#a6e3a1", "word-break": "break-word" }}>{settingsMsg()}</div>
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
                  <For each={blocks()}>
                    {(b) => <BlockNode b={b} recordId={r.id} depth={0} />}
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
