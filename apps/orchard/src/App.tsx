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
  if (kind && typeof kind === "object" && kind.Heading) return "H" + kind.Heading.level;
  if (typeof kind === "string") return kind;
  return "block";
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
        <select
          value={iconChoice()}
          onChange={(e) => setIconChoice(e.currentTarget.value)}
          style={{ padding: "4px", "border-radius": "6px", background: "#313244", color: "#cdd6f4", border: "1px solid #45475a" }}
        >
          <option value="flower">Flower (top view)</option>
          <option value="sideways">Flower (side view)</option>
        </select>
        <input
          type="text"
          placeholder="search (tag:rust, type:page, ...)"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") load(); }}
        />
        <button onClick={() => load()}>Search</button>
      </header>
      <div style={{ "margin-bottom": "12px", display: "flex", gap: "8px", "align-items": "center", "flex-wrap": "wrap" }}>
        <input id="extid" placeholder="Chrome extension ID (for host registration)" style={{ width: "360px", padding: "6px 10px", "border-radius": "6px", border: "1px solid #45475a", background: "#313244", color: "#cdd6f4" }} />
        <button onClick={async () => {
          const id = (document.getElementById("extid") as HTMLInputElement)?.value || "";
          try {
            const r = await invoke("register_native_host", { extensionId: id, hostBinaryPath: null });
            alert(r);
          } catch (e) { alert("Error: " + e); }
        }}>Register native host</button>
        <span style={{ "font-size": "12px", color: "#a6adc8" }}>Registers lilac-host for Chrome + Firefox.</span>
      </div>
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
                    {(b) => (
                      <div style={{ "margin-bottom": "6px" }}>
                        <b>{kindLabel(b.kind)}</b>
                        <CodeMirrorBlock
                          initial={b.content || ""}
                          onSave={(v) => applyBlockEdit(r.id, b.id!, v).catch(() => {})}
                        />
                      </div>
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
