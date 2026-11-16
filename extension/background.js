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

// ---------------------------------------------------------------------------
// Browser context hints
//
// Hints, not truth: they help Orchard propose grouping/placement, but no
// browser window or tab id is ever written to a record, and a record stays
// valid if the window is gone. Cross-browser rule: only url/title are
// authoritative.
// ---------------------------------------------------------------------------

function isWebUrl(url) {
  return typeof url === "string" && /^https?:/i.test(url);
}

//// Per-tab hints. `tab_index` is a position within the window, not an id.
async function tabHints(tabs) {
  const byWindow = new Map();
  for (const t of tabs) {
    if (!byWindow.has(t.windowId)) byWindow.set(t.windowId, []);
    byWindow.get(t.windowId).push(t);
  }
  const hints = new Map();
  for (const [windowId, subset] of byWindow) {
    let siblings = subset;
    let incognito = false;
    try {
      const win = await chrome.windows.get(windowId);
      incognito = !!win.incognito;
      siblings = await chrome.tabs.query({ windowId });
    } catch (e) {
      // Firefox may hide windows; the subset still gives a usable count.
    }
    for (const t of subset) {
      const index = siblings.findIndex((s) => s.id === t.id);
      const hint = { pinned: !!t.pinned, incognito, tab_count: siblings.length };
      if (index >= 0) hint.tab_index = index;
      hints.set(t.id, hint);
    }
  }
  return hints;
}

async function windowSummary(windowId, selectedCount) {
  try {
    const win = await chrome.windows.get(windowId);
    const siblings = await chrome.tabs.query({ windowId });
    return { tab_count: siblings.length, selected_count: selectedCount, incognito: !!win.incognito };
  } catch (e) {
    return { selected_count: selectedCount };
  }
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
        if (msg.folder) record.folder = msg.folder;
        const hints = await tabHints([t]);
        if (hints.has(t.id)) record.window_hint = hints.get(t.id);
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
            const hints = await tabHints([t2]);
            if (hints.has(t2.id)) record.window_hint = hints.get(t2.id);
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
            const hints = await tabHints([t]);
            if (hints.has(t.id)) record.window_hint = hints.get(t.id);
            const rec = await send("save_record", { record, body: "" });
            sendResponse({ ok: true, result: { record: rec, asset } });
          } catch (e) {
            sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
          }
        });
        return true;
      } else if (msg.type === "saveSelectedTabs") {
        // Only tabs the user actually highlighted. Falling back to the whole
        // window made "save selected" quietly save everything.
        const tabs = await chrome.tabs.query({ highlighted: true, currentWindow: true });
        const list = (tabs || []).filter((t) => isWebUrl(t.url));
        if (!list.length) {
          throw new Error("select at least one http(s) tab first");
        }
        const folder = msg.folder || null;
        const hints = await tabHints(list);
        // capture shared tab group (if all selected tabs share one group)
        let sharedGroup = null;
        const groupIds = new Set(list.map((t) => t.groupId).filter((g) => g && g !== -1));
        if (groupIds.size === 1 && chrome.tabGroups && chrome.tabGroups.get) {
          try { sharedGroup = await chrome.tabGroups.get([...groupIds][0]); } catch (e) {}
        }
        // Save each tab as a page and keep one structured entry per page, so the
        // collection can show titles, groups and order without re-reading the
        // body or guessing from links.
        const entries = [];
        for (const t of list) {
          const record = { type: "page", title: t.title || "", url: t.url || "", tags: [] };
          if (folder) record.folder = folder;
          if (hints.has(t.id)) record.window_hint = hints.get(t.id);
          let group = null;
          if (t.groupId && t.groupId !== -1 && chrome.tabGroups && chrome.tabGroups.get) {
            try {
              const g = await chrome.tabGroups.get(t.groupId);
              if (g) {
                group = { title: g.title || "", color: g.color || "" };
                record.tab_group = group;
              }
            } catch (e) {}
          }
          const r = await send("save_record", { record, body: "" });
          const entry = { id: r.id, url: t.url || "", title: t.title || "", pinned: !!t.pinned };
          if (group) {
            entry.group = group.title;
            entry.group_color = group.color;
          }
          entries.push(entry);
        }
        // The collection is its own record type. Putting it in the page folder
        // (the usual default) made it appear as a page and buried it.
        const body = entries.map((entry) => "- [[" + entry.id + "]]").join("\n");
        const label = sharedGroup && sharedGroup.title
          ? "Saved tabs — " + sharedGroup.title
          : "Saved tabs " + new Date().toLocaleString();
        const col = {
          type: "collection",
          title: label,
          tags: [],
          items: entries,
          saved_from: sharedGroup ? "group" : "selection",
        };
        if (sharedGroup) col.tab_group = { title: sharedGroup.title || "", color: sharedGroup.color || "" };
        const summary = await windowSummary(list[0].windowId, list.length);
        if (summary) col.window_hint = summary;
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
        const payload = { record: msg.record };
        if (msg.rule) payload.rule = msg.rule;
        const res = await send("derive_title", payload);
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
      } else if (msg.type === "setRegisteredFolders") {
        const res = await send("set_registered_folders", { folders: msg.folders || [] });
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
        // Opening is not editing: the host touches only `opened`.
        try { await send("touch_opened", { id: msg.id }); } catch (e) {}
        sendResponse({ ok: true });
      } else if (msg.type === "deleteRecord") {
        await send("delete_record", { id: msg.id });
        sendResponse({ ok: true });
      } else if (msg.type === "reopenCollection") {
        const res = await send("load_record", { id: msg.id });
        const rec = res && res.record;
        // Structured entries are authoritative; older collections are read from
        // the body so they still open.
        let entries = rec && Array.isArray(rec.items) ? rec.items.slice() : [];
        const body = res && res.body;
        if (!entries.length && body) {
          for (const line of body.split("\n")) {
            const m = line.match(/\[\[(.+?)\]\]/);
            if (m) {
              try {
                const r = await send("load_record", { id: m[1] });
                if (r && r.record) entries.push({ id: m[1], url: r.record.url || "", title: r.record.title || "" });
              } catch (e) {}
            } else {
              const u = line.trim().replace(/^[-*]\s*/, "");
              if (u.startsWith("http")) entries.push({ url: u });
            }
          }
        }
        // Fill missing URLs from the page records without dropping order.
        for (const entry of entries) {
          if (!entry.url && entry.id) {
            try {
              const r = await send("load_record", { id: entry.id });
              if (r && r.record) {
                entry.url = r.record.url || "";
                entry.title = entry.title || r.record.title || "";
              }
            } catch (e) {}
          }
        }
        const opened = entries.filter((entry) => entry.url);
        const createdTabs = [];
        for (const entry of opened) {
          const t = await chrome.tabs.create({ url: entry.url, active: false, pinned: !!entry.pinned });
          createdTabs.push(t);
        }
        // Opening the set and its pages is a view, not an edit: touch only
        // `opened`, and only for records that were actually opened.
        try { await send("touch_opened", { id: msg.id }); } catch (e) {}
        for (const entry of opened) {
          if (entry.id) {
            try { await send("touch_opened", { id: entry.id }); } catch (e) {}
          }
        }
        // Rebuild the group shape: one group for the whole set when the
        // collection captured a shared group, otherwise per entry.
        const tg = rec && rec.tab_group;
        if (tg && createdTabs.length && chrome.tabs.group && chrome.tabGroups && chrome.tabGroups.update) {
          try {
            const groupId = await chrome.tabs.group({ tabIds: createdTabs.map((t) => t.id) });
            await chrome.tabGroups.update(groupId, { title: tg.title || "", color: tg.color || "grey" });
          } catch (e) {}
        } else if (chrome.tabs.group && chrome.tabGroups && chrome.tabGroups.update) {
          const byGroup = new Map();
          opened.forEach((entry, i) => {
            if (!entry.group) return;
            if (!byGroup.has(entry.group)) byGroup.set(entry.group, { color: entry.group_color || "grey", tabs: [] });
            byGroup.get(entry.group).tabs.push(createdTabs[i]);
          });
          for (const [title, info] of byGroup) {
            try {
              const groupId = await chrome.tabs.group({ tabIds: info.tabs.map((t) => t.id) });
              await chrome.tabGroups.update(groupId, { title, color: info.color });
            } catch (e) {}
          }
        }
        sendResponse({ ok: true, count: createdTabs.length });
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
    const hints = await tabHints([t]);
    if (hints.has(t.id)) record.window_hint = hints.get(t.id);
    if (t.groupId && t.groupId !== -1 && chrome.tabGroups && chrome.tabGroups.get) {
      try {
        const g = await chrome.tabGroups.get(t.groupId);
        if (g) record.tab_group = { title: g.title || "", color: g.color || "" };
      } catch (e) {}
    }
    await send("save_record", { record, body: "" });
  } else if (command === "open-manager") {
    const url = chrome.runtime.getURL("lilac.html");
    await chrome.tabs.create({ url });
  }
});