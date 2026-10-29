import { createSignal, For, Show } from "solid-js";
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

export default function App() {
  const [records, setRecords] = createSignal<Record[]>([]);
  const [query, setQuery] = createSignal("");
  const [expanded, setExpanded] = createSignal<string | null>(null);
  const [blocks, setBlocks] = createSignal<Block[]>([]);

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

  return (
    <>
      <header>
        <h1>Orchard</h1>
        <input
          type="text"
          placeholder="search (tag:rust, type:page, ...)"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") load(); }}
        />
        <button onClick={() => load()}>Search</button>
      </header>
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
