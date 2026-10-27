// Lilac service worker: native host bridge.
// Keeps a single native port; queues calls while disconnected.

const HOST_NAME = "lilac_host";

let port = null;
let connecting = false;
const pending = new Map(); // id -> {resolve, reject}
let seq = 0;

function ensurePort() {
  if (port) return port;
  if (connecting) return null;
  connecting = true;
  try {
    port = chrome.runtime.connectNative(HOST_NAME);
  } catch (e) {
    connecting = false;
    console.error("connectNative failed", e);
    return null;
  }
  connecting = false;
  port.onMessage.addListener((msg) => {
    const id = msg && msg.id;
    if (id != null && pending.has(id)) {
      const { resolve, reject } = pending.get(id);
      pending.delete(id);
      if (msg.ok) resolve(msg.result);
      else reject(new Error((msg.error && msg.error.message) || "host error"));
    }
  });
  port.onDisconnect.addListener(() => {
    const err = chrome.runtime.lastError ? chrome.runtime.lastError.message : "disconnected";
    console.warn("native port disconnected:", err);
    port = null;
    // reject all pending
    for (const [id, { reject }] of pending) {
      reject(new Error("host disconnected"));
    }
    pending.clear();
  });
  return port;
}

function send(method, params) {
  return new Promise((resolve, reject) => {
    const p = ensurePort();
    if (!p) {
      reject(new Error("native host not available"));
      return;
    }
    const id = ++seq;
    pending.set(id, { resolve, reject });
    try {
      p.postMessage({ id, method, params: params || {} });
    } catch (e) {
      pending.delete(id);
      reject(e);
    }
    // timeout
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("host timeout"));
      }
    }, 30000);
  });
}

// Public API for popup via runtime messages.
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (!msg || !msg.type) return;
  (async () => {
    try {
      if (msg.type === "saveCurrentTab") {
        const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
        const t = tabs && tabs[0];
        if (!t) throw new Error("no active tab");
        const record = {
          type: "page",
          title: t.title || "",
          url: t.url || "",
          tags: [],
        };
        const res = await send("save_record", { record, body: "" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "saveSelectedTabs") {
        const tabs = await chrome.tabs.query({ highlighted: true, currentWindow: true });
        const list = tabs && tabs.length ? tabs : await chrome.tabs.query({ currentWindow: true });
        // save each as page
        const ids = [];
        for (const t of list) {
          const record = { type: "page", title: t.title || "", url: t.url || "", tags: [] };
          const r = await send("save_record", { record, body: "" });
          ids.push(r.id);
        }
        // create collection
        const body = ids.map((id) => "- [[" + id + "]]").join("\n");
        const col = { type: "collection", title: "Saved tabs " + new Date().toISOString(), tags: [] };
        const cres = await send("save_record", { record: col, body });
        sendResponse({ ok: true, result: { count: list.length, collection: cres } });
      } else if (msg.type === "listRecent") {
        const res = await send("search_records", { query: "" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "getConfig") {
        const res = await send("get_config", {});
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "openFolder") {
        const res = await send("open_folder", { path: msg.which || "records" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "reopenRecord") {
        const res = await send("load_record", { id: msg.id });
        const rec = res && res.record;
        const url = rec && rec.url;
        if (url) await chrome.tabs.create({ url, active: false });
        sendResponse({ ok: true });
      } else if (msg.type === "reopenCollection") {
        const res = await send("load_record", { id: msg.id });
        const body = res && res.body;
        const urls = [];
        if (body) {
          for (const line of body.split("\n")) {
            const m = line.match(/\[\[(.+?)\]\]/);
            if (m) {
              try {
                const r = await send("load_record", { id: m[1] });
                if (r && r.record && r.record.url) urls.push(r.record.url);
              } catch (e) {}
            } else {
              const u = line.trim().replace(/^[-*]\s*/, "");
              if (u.startsWith("http")) urls.push(u);
            }
          }
        }
        for (const u of urls) await chrome.tabs.create({ url: u, active: false });
        sendResponse({ ok: true, count: urls.length });
      } else {
        sendResponse({ ok: false, error: "unknown type" });
      }
    } catch (e) {
      sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
    }
  })();
  return true; // async
});