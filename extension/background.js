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
        // capture tab group metadata (hint; cross-browser rule: not authoritative)
        if (t.groupId && t.groupId !== -1 && chrome.tabGroups && chrome.tabGroups.get) {
          try {
            const g = await chrome.tabGroups.get(t.groupId);
            if (g) record.tab_group = { title: g.title || "", color: g.color || "" };
          } catch (e) {}
        }
        const res = await send("save_record", { record, body: "" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "snapshotCurrentTab") {
        // Chrome-only: capture current tab as MHTML and store as an asset.
        if (!chrome.pageCapture || !chrome.pageCapture.saveAsMHTML) {
          // Fallback: extract HTML via content script (cross-browser).
          try {
            const tabs2 = await chrome.tabs.query({ active: true, currentWindow: true });
            const t2 = tabs2 && tabs2[0];
            if (!t2) throw new Error("no active tab");
            const htmlResp = await new Promise((resolve) => {
              chrome.tabs.sendMessage(t2.id, { type: "extractHtml" }, (r) => resolve(r));
            });
            if (!htmlResp || !htmlResp.ok) throw new Error((htmlResp && htmlResp.error) || "extract failed");
            const b64 = btoa(unescape(encodeURIComponent(htmlResp.html)));
            const asset = await send("save_asset", { data: b64, ext: "html", subdir: "Assets" });
            const record = { type: "page", title: htmlResp.title || t2.title || "", url: htmlResp.url || t2.url || "", tags: [], snapshot: asset.path, snapshot_format: "html" };
            const rec = await send("save_record", { record, body: "" });
            sendResponse({ ok: true, result: { record: rec, asset } });
          } catch (e) {
            sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
          }
          return;
        }
        const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
        const t = tabs && tabs[0];
        if (!t) { sendResponse({ ok: false, error: "no active tab" }); return; }
        chrome.pageCapture.saveAsMHTML({ tabId: t.id }, async (blob) => {
          if (chrome.runtime.lastError || !blob) {
            sendResponse({ ok: false, error: (chrome.runtime.lastError && chrome.runtime.lastError.message) || "capture failed" });
            return;
          }
          try {
            const buf = await blob.arrayBuffer();
            const bytes = new Uint8Array(buf);
            let bin = "";
            for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
            const b64 = btoa(bin);
            const asset = await send("save_asset", { data: b64, ext: "mhtml", subdir: "Assets" });
            // save a page record referencing the snapshot
            const record = { type: "page", title: t.title || "", url: t.url || "", tags: [], snapshot: asset.path, snapshot_format: "mhtml" };
            const rec = await send("save_record", { record, body: "" });
            sendResponse({ ok: true, result: { record: rec, asset } });
          } catch (e) {
            sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
          }
        });
        return true;
      } else if (msg.type === "saveSelectedTabs") {
        const tabs = await chrome.tabs.query({ highlighted: true, currentWindow: true });
        const list = tabs && tabs.length ? tabs : await chrome.tabs.query({ currentWindow: true });
        // capture shared tab group (if all selected tabs share one group)
        let sharedGroup = null;
        const groupIds = new Set(list.map((t) => t.groupId).filter((g) => g && g !== -1));
        if (groupIds.size === 1 && chrome.tabGroups && chrome.tabGroups.get) {
          try { sharedGroup = await chrome.tabGroups.get([...groupIds][0]); } catch (e) {}
        }
        // save each as page
        const ids = [];
        for (const t of list) {
          const record = { type: "page", title: t.title || "", url: t.url || "", tags: [] };
          if (t.groupId && t.groupId !== -1 && chrome.tabGroups && chrome.tabGroups.get) {
            try {
              const g = await chrome.tabGroups.get(t.groupId);
              if (g) record.tab_group = { title: g.title || "", color: g.color || "" };
            } catch (e) {}
          }
          const r = await send("save_record", { record, body: "" });
          ids.push(r.id);
        }
        // create collection
        const body = ids.map((id) => "- [[" + id + "]]").join("\n");
        const col = { type: "collection", title: "Saved tabs " + new Date().toISOString(), tags: [] };
        if (sharedGroup) col.tab_group = { title: sharedGroup.title || "", color: sharedGroup.color || "" };
        const cres = await send("save_record", { record: col, body });
        sendResponse({ ok: true, result: { count: list.length, collection: cres } });
      } else if (msg.type === "loadRecord") {
        const res = await send("load_record", { id: msg.id });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "listRecent") {
        const res = await send("search_records", { query: "" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "search") {
        const res = await send("search_records", { query: msg.query || "" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "deriveTitle") {
        const res = await send("derive_title", { record: msg.record, rule: msg.rule });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "applyRules") {
        const res = await send("apply_rules", { record: msg.record, rules: msg.rules });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "updateRecord") {
        const patch = msg.patch || {};
        const res = await send("update_record", { id: msg.id, patch, body: msg.body });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "listConflicts") {
        const res = await send("list_conflicts", {});
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "rebuildIndex") {
        const res = await send("rebuild_index", {});
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "getConfig") {
        const res = await send("get_config", {});
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "setRecordsRoot") {
        const res = await send("set_records_root", { path: msg.path });
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
      } else if (msg.type === "deleteRecord") {
        await send("delete_record", { id: msg.id });
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
        const createdTabs = [];
        for (const u of urls) {
          const t = await chrome.tabs.create({ url: u, active: false });
          createdTabs.push(t);
        }
        const tg = res && res.record && res.record.tab_group;
        if (tg && createdTabs.length && chrome.tabs.group && chrome.tabGroups && chrome.tabGroups.update) {
          try {
            const groupId = await chrome.tabs.group({ tabIds: createdTabs.map((t) => t.id) });
            await chrome.tabGroups.update(groupId, { title: tg.title || "", color: tg.color || "grey" });
          } catch (e) {}
        }
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

// Keyboard shortcuts
chrome.commands.onCommand.addListener(async (command) => {
  if (command === "save-tab") {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    const t = tabs && tabs[0];
    if (!t) return;
    const record = { type: "page", title: t.title || "", url: t.url || "", tags: [] };
    await send("save_record", { record, body: "" });
  } else if (command === "open-manager") {
    const url = chrome.runtime.getURL("lilac.html");
    await chrome.tabs.create({ url });
  }
});