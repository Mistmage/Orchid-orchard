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
    const hint = {
      tab_count: siblings.length,
      selected_count: selectedCount,
      incognito: !!win.incognito,
    };
    // Geometry is a hint like everything else: an OS may place the window
    // somewhere else. Workspaces (T13) turn the same values into a name.
    for (const key of ["left", "top", "width", "height"]) {
      if (typeof win[key] === "number") hint[key] = win[key];
    }
    if (win.state) hint.state = win.state;
    return hint;
  } catch (e) {
    return { selected_count: selectedCount };
  }
}

/// Group metadata in the order groups first appear among `tabs`. The browser's
/// group id lives only inside this function; a record must stay meaningful in
/// another session, so it stores name, colour and collapsed state instead.
async function tabGroupsFor(tabs) {
  const groups = [];
  const byId = new Map();
  for (const t of tabs) {
    const id = t.groupId;
    if (!id || id === -1 || byId.has(id)) continue;
    let meta = null;
    if (chrome.tabGroups && chrome.tabGroups.get) {
      try { meta = await chrome.tabGroups.get(id); } catch (e) {}
    }
    const hint = {
      name: (meta && meta.title) || "",
      color: (meta && meta.color) || "",
      collapsed: !!(meta && meta.collapsed),
    };
    byId.set(id, hint);
    groups.push(hint);
  }
  return { groups, byId };
}

/// One tab as the save planner sees it: authoritative url/title plus browser
/// hints. Ids never enter the plan; only names, order and state do.
function planTabOf(t, hints, groupsById) {
  const tab = {
    url: t.url || "",
    title: t.title || "",
    pinned: !!t.pinned,
    active: !!t.active,
    muted: !!t.muted,
    extra: {},
  };
  const group = groupsById.get(t.groupId) || null;
  if (group) {
    tab.group = group.name;
    tab.group_color = group.color;
    tab.group_collapsed = !!group.collapsed;
    tab.extra.tab_group = { title: group.name, color: group.color };
  }
  const hint = hints.get(t.id);
  if (hint) tab.extra.window_hint = hint;
  return tab;
}

/// Resolve a save source into the tabs it means. Preview and commit share this,
/// so they can never disagree about what a scope captured.
async function tabsForSource(source) {
  let tabs = [];
  let windowId = null;
  if (source === "page" || source === "snapshot") {
    const active = await chrome.tabs.query({ active: true, currentWindow: true });
    const t = active && active[0];
    if (!t) throw new Error("no active tab");
    tabs = [t];
    windowId = t.windowId;
  } else if (source === "selection") {
    // Only tabs the user actually highlighted: falling back to the whole
    // window would make "save selected" quietly save everything.
    tabs = await chrome.tabs.query({ highlighted: true, currentWindow: true });
    windowId = tabs.length ? tabs[0].windowId : null;
  } else if (source === "group") {
    const active = await chrome.tabs.query({ active: true, currentWindow: true });
    const t = active && active[0];
    if (!t) throw new Error("no active tab");
    if (!t.groupId || t.groupId === -1) {
      throw new Error("the active tab is not in a tab group");
    }
    windowId = t.windowId;
    const inWindow = await chrome.tabs.query({ windowId });
    tabs = inWindow.filter((candidate) => candidate.groupId === t.groupId);
  } else if (source === "window" || source === "workspace") {
    const active = await chrome.tabs.query({ active: true, currentWindow: true });
    const t = active && active[0];
    if (!t) throw new Error("no active window");
    windowId = t.windowId;
    tabs = await chrome.tabs.query({ windowId });
  } else {
    throw new Error("unknown save source: " + source);
  }
  const ordered = (tabs || [])
    .filter((t) => isWebUrl(t.url))
    .sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
  if (!ordered.length) throw new Error("there are no http(s) tabs to save");
  const hints = await tabHints(ordered);
  const { byId } = await tabGroupsFor(ordered);
  return {
    windowId,
    tabs: ordered.map((t) => planTabOf(t, hints, byId)),
  };
}

/// Snapshot bytes are an asset, not a record: capture them before the plan so
/// the preview can show the file the record will point at. A content-addressed
/// asset written and then cancelled is harmless.
async function captureSnapshot(tab) {
  const base64Of = async (blob) => {
    const bytes = new Uint8Array(await blob.arrayBuffer());
    let bin = "";
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin);
  };
  if (chrome.pageCapture && chrome.pageCapture.saveAsMHTML) {
    try {
      const blob = await new Promise((resolve, reject) => {
        chrome.pageCapture.saveAsMHTML({ tabId: tab.id }, (captured) => {
          if (chrome.runtime.lastError) {
            reject(new Error(chrome.runtime.lastError.message || "capture failed"));
          } else if (!captured) {
            reject(new Error("capture failed"));
          } else {
            resolve(captured);
          }
        });
      });
      const asset = await send("save_asset", {
        data: await base64Of(blob),
        ext: "mhtml",
        subdir: "Assets",
      });
      return { path: asset.path, format: "mhtml", title: tab.title, url: tab.url };
    } catch (e) {
      // Fall through to the cross-browser HTML extraction.
    }
  }
  const extracted = await new Promise((resolve) => {
    chrome.tabs.sendMessage(tab.id, { type: "extractHtml" }, (resp) => resolve(resp));
  });
  if (!extracted || !extracted.ok) {
    throw new Error((extracted && extracted.error) || "could not capture this page");
  }
  const asset = await send("save_asset", {
    data: btoa(unescape(encodeURIComponent(extracted.html))),
    ext: "html",
    subdir: "Assets",
  });
  return {
    path: asset.path,
    format: "html",
    title: extracted.title || tab.title,
    url: extracted.url || tab.url,
  };
}

/// Build the plan a preview will show and a commit will write. Nothing here
/// writes a record; a workspace may describe its display, which is a lookup.
async function startSaveFor(msg) {
  const source = msg.source || "page";
  const { windowId, tabs } = await tabsForSource(source);
  const request = {
    source,
    tabs,
    folder: msg.folder || null,
    title: msg.title || null,
    description: msg.description || null,
  };
  if (source === "workspace") {
    const win = await chrome.windows.get(windowId);
    let geometry = geometryFromWindow(win);
    const displays = await displayInfos();
    if (displays.length && geometry.left != null) {
      try {
        const described = await send("describe_display", { geometry, displays });
        if (described && described.geometry) geometry = described.geometry;
      } catch (e) {}
    }
    request.geometry = geometry;
  }
  if (source === "snapshot") {
    const active = await chrome.tabs.query({ active: true, currentWindow: true });
    const captured = await captureSnapshot(active && active[0]);
    request.source = "page";
    request.tabs[0].snapshot = captured.path;
    request.tabs[0].snapshot_format = captured.format;
    request.tabs[0].title = captured.title || request.tabs[0].title;
    request.tabs[0].url = captured.url || request.tabs[0].url;
  }
  const plan = await send("plan_save", request);
  return { plan, source };
}

/// A set's body mirrors its entries as links, so the file stays readable.
function setBody(entries) {
  return entries.map((entry) => "- [[" + entry.id + "]]").join("\n");
}

/// Save page records for a set of tabs and return its ordered entries. When
/// `existing` entries are given (a workspace being updated), a tab whose URL
/// was already captured reuses that page record instead of creating a second
/// one; order still follows the live window.
async function savePagesFor(tabs, folder, existing) {
  const byUrl = new Map();
  for (const entry of existing || []) {
    if (entry && entry.url) byUrl.set(entry.url, entry);
  }
  const hints = await tabHints(tabs);
  const { groups, byId } = await tabGroupsFor(tabs);
  const entries = [];
  for (const t of tabs) {
    const group = byId.get(t.groupId) || null;
    const prior = byUrl.get(t.url || "");
    if (prior && prior.id) {
      const entry = Object.assign({}, prior, {
        url: t.url || prior.url,
        title: t.title || prior.title || "",
        pinned: !!t.pinned,
        active: !!t.active,
        muted: !!t.muted,
      });
      if (group) {
        entry.group = group.name;
        entry.group_color = group.color;
      } else {
        delete entry.group;
        delete entry.group_color;
      }
      entries.push(entry);
      continue;
    }
    const record = { type: "page", title: t.title || "", url: t.url || "", tags: [] };
    if (folder) record.folder = folder;
    if (hints.has(t.id)) record.window_hint = hints.get(t.id);
    if (group) record.tab_group = { title: group.name, color: group.color };
    const saved = await send("save_record", { record, body: "" });
    const entry = {
      id: saved.id,
      url: t.url || "",
      title: t.title || "",
      pinned: !!t.pinned,
      active: !!t.active,
      muted: !!t.muted,
    };
    if (group) {
      entry.group = group.name;
      entry.group_color = group.color;
    }
    entries.push(entry);
  }
  return { entries, groups };
}

/// Save a set of tabs as page records plus one collection. Every scope — a
/// selection, one group, a whole window — goes through this path so the three
/// cannot drift into slightly different shapes.
async function saveTabSet(tabs, options) {
  const { folder = null, source = "selection", title = null } = options || {};
  const list = (tabs || []).filter((t) => isWebUrl(t.url));
  if (!list.length) throw new Error("select at least one http(s) tab first");
  const ordered = list.slice().sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
  const { entries, groups } = await savePagesFor(ordered, folder);
  const body = setBody(entries);
  const named = groups.find((g) => g.name);
  const label = title || (named
    ? "Saved tabs — " + named.name
    : "Saved tabs " + new Date().toLocaleString());
  const collection = {
    type: "collection",
    title: label,
    tags: [],
    items: entries,
    saved_from: source,
  };
  // A single group is also recorded at the set level, which is what older
  // builds and the plain "open all" path understand.
  if (groups.length === 1) {
    collection.tab_group = { title: groups[0].name, color: groups[0].color };
  }
  if (groups.length) collection.tab_groups = groups;
  const summary = await windowSummary(ordered[0].windowId, ordered.length);
  if (summary) collection.window_hint = summary;
  const saved = await send("save_record", { record: collection, body });
  return { count: entries.length, collection: saved, groups: groups.length };
}

/// Chrome reports outer bounds for the current window state. Keep only what it
/// actually gave us instead of writing zeros for missing values.
function geometryFromWindow(win) {
  const geometry = {};
  for (const key of ["left", "top", "width", "height"]) {
    if (typeof win[key] === "number") geometry[key] = win[key];
  }
  if (win.state) geometry.state = win.state;
  return geometry;
}

/// Visible displays, in the shape the core's geometry rules expect. A browser
/// without `system.display` simply reports none, and reopening then applies the
/// saved bounds without checking them.
async function displayInfos() {
  if (!(chrome.system && chrome.system.display && chrome.system.display.getInfo)) return [];
  try {
    const infos = await chrome.system.display.getInfo();
    const rectOf = (r) => (r
      ? { left: r.left, top: r.top, width: r.width, height: r.height }
      : null);
    return (infos || [])
      .map((d) => ({
        bounds: rectOf(d.bounds),
        work_area: rectOf(d.workArea),
        scale: typeof d.scaleFactor === "number" ? d.scaleFactor : null,
        primary: !!d.isPrimary,
      }))
      .filter((d) => d.bounds);
  } catch (e) {
    return [];
  }
}

/// Capture the current window as a workspace: its tabs, its groups and the
/// display it was on. Which display that is and where the window may reopen is
/// the core's decision, not this file's, so the host describes and clamps it.
async function captureWindow(windowId, options) {
  const { folder = null, existing = null } = options || {};
  const win = await chrome.windows.get(windowId);
  const all = await chrome.tabs.query({ windowId });
  const ordered = (all || [])
    .filter((t) => isWebUrl(t.url))
    .sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
  if (!ordered.length) throw new Error("this window has no http(s) tabs to save");
  const { entries, groups } = await savePagesFor(ordered, folder, existing);
  let geometry = geometryFromWindow(win);
  const displays = await displayInfos();
  if (displays.length && geometry.left != null) {
    try {
      const res = await send("describe_display", { geometry, displays });
      if (res && res.geometry) geometry = res.geometry;
    } catch (e) {}
  }
  return {
    win,
    entries,
    groups,
    geometry,
    body: setBody(entries),
    tabCount: ordered.length,
  };
}

/// The window state a workspace may ask back. `minimized` is not restored:
/// opening a workspace into a minimised window would hide the result. Chromium
/// reports `locked-fullscreen`; plain fullscreen is what can be requested.
function applyableState(state) {
  if (state === "maximized" || state === "fullscreen") return state;
  if (state === "locked-fullscreen") return "fullscreen";
  return null;
}

/// Groups captured on a set, with the old single-group field as a fallback.
function groupHintsOf(rec) {
  if (rec && Array.isArray(rec.tab_groups) && rec.tab_groups.length) return rec.tab_groups;
  if (rec && rec.tab_group) {
    return [{ name: rec.tab_group.title || "", color: rec.tab_group.color || "", collapsed: false }];
  }
  return [];
}

/// Put freshly created tabs back into the groups the set described. Returns
/// what happened so the popup can report a degraded open instead of silently
/// pretending the shape was restored.
async function rebuildGroups(tabs, entries, groupHints, mode, targetGroupId) {
  if (!tabs.length) return { grouped: false, warning: null };
  if (!chrome.tabs.group || !chrome.tabGroups || !chrome.tabGroups.update) {
    return {
      grouped: false,
      warning: "this browser does not support tab groups; the tabs were opened ungrouped",
    };
  }
  if (mode === "into") {
    if (targetGroupId == null) {
      return { grouped: false, warning: "no target group was chosen; the tabs were opened ungrouped" };
    }
    try {
      await chrome.tabs.group({ tabIds: tabs.map((t) => t.id), groupId: targetGroupId });
      return { grouped: true, warning: null };
    } catch (e) {
      return {
        grouped: false,
        warning: "could not add the tabs to the chosen group: " + (e && e.message ? e.message : e),
      };
    }
  }
  const byName = new Map();
  entries.forEach((entry, i) => {
    const name = entry.group || "";
    if (!name) return;
    if (!byName.has(name)) {
      byName.set(name, { color: entry.group_color || "grey", tabs: [] });
    }
    byName.get(name).tabs.push(tabs[i]);
  });
  // A legacy single-group collection has no per-entry group, only the set's.
  if (!byName.size && groupHints && groupHints.length && groupHints[0].name) {
    byName.set(groupHints[0].name, { color: groupHints[0].color || "grey", tabs });
  }
  if (!byName.size) return { grouped: false, warning: null };
  const order = (groupHints || []).map((g) => g.name);
  const names = [...byName.keys()].sort((a, b) => {
    const ai = order.indexOf(a);
    const bi = order.indexOf(b);
    return (ai < 0 ? 999 : ai) - (bi < 0 ? 999 : bi);
  });
  let lastError = null;
  for (const name of names) {
    const info = byName.get(name);
    const hint = (groupHints || []).find((g) => g.name === name);
    try {
      const groupId = await chrome.tabs.group({ tabIds: info.tabs.map((t) => t.id) });
      const update = { title: name, color: info.color || "grey" };
      if (hint && typeof hint.collapsed === "boolean") update.collapsed = hint.collapsed;
      await chrome.tabGroups.update(groupId, update);
    } catch (e) {
      lastError = e;
    }
  }
  return lastError
    ? {
        grouped: true,
        warning: "some groups could not be rebuilt: " + (lastError.message || lastError),
      }
    : { grouped: true, warning: null };
}

/// Load a set record and resolve its entries to openable URLs. Structured
/// entries are authoritative; a body without them (older collections) is read
/// as links and plain URLs, so old files still open.
async function loadSet(id) {
  const res = await send("load_record", { id });
  const rec = res && res.record;
  if (!rec) throw new Error("record not found: " + id);
  let entries = Array.isArray(rec.items) ? rec.items.slice() : [];
  const body = res && res.body;
  if (!entries.length && body) {
    for (const line of body.split("\n")) {
      const m = line.match(/\[\[(.+?)\]\]/);
      if (m) {
        try {
          const r = await send("load_record", { id: m[1] });
          if (r && r.record) {
            entries.push({ id: m[1], url: r.record.url || "", title: r.record.title || "" });
          }
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
  return { rec, entries };
}

/// Open a set's tabs in captured order, then restore groups, focus and window
/// state. Shared by collections and workspaces so reopening cannot drift.
async function openTabSet(rec, entries, options) {
  const { mode = "group", targetGroupId = null, placement = null } = options || {};
  const opened = entries.filter((entry) => entry.url);
  if (!opened.length) throw new Error("this set has no openable URLs");
  const warnings = [];
  const createdTabs = [];
  let win = null;
  if (mode === "window") {
    // A new window is the honest way to reopen a captured window: its groups
    // belong to it, not to whatever window the popup was in.
    const createOptions = { url: opened[0].url, focused: true };
    if (placement) {
      createOptions.left = Math.round(placement.left);
      createOptions.top = Math.round(placement.top);
      createOptions.width = Math.round(placement.width);
      createOptions.height = Math.round(placement.height);
    }
    try {
      win = await chrome.windows.create(createOptions);
    } catch (e) {
      if (!placement) throw e;
      // Bounds are a hint: an OS that refuses them must not stop the open.
      warnings.push("the browser refused the saved bounds; the window opened at the default position");
      win = await chrome.windows.create({ url: opened[0].url, focused: true });
    }
    const first = win && win.tabs && win.tabs[0];
    if (!first) throw new Error("the browser did not return the new window's tab");
    if (opened[0].muted) {
      try { await chrome.tabs.update(first.id, { muted: true }); } catch (e) {}
    }
    createdTabs.push(first);
    for (let i = 1; i < opened.length; i++) {
      const t = await chrome.tabs.create({
        windowId: win.id,
        url: opened[i].url,
        index: i,
        active: false,
        pinned: !!opened[i].pinned,
        muted: !!opened[i].muted,
      });
      createdTabs.push(t);
    }
  } else {
    for (const entry of opened) {
      const t = await chrome.tabs.create({
        url: entry.url,
        active: false,
        pinned: !!entry.pinned,
        muted: !!entry.muted,
      });
      createdTabs.push(t);
    }
  }
  // Restore focus last, so creating the other tabs does not steal it.
  const activeIndex = opened.findIndex((entry) => entry.active);
  if (activeIndex >= 0 && createdTabs[activeIndex]) {
    try {
      await chrome.tabs.update(createdTabs[activeIndex].id, { active: true });
    } catch (e) {}
  }
  // Opening the set and its pages is a view, not an edit: touch only `opened`,
  // and only for records that were actually opened.
  try { await send("touch_opened", { id: rec.id }); } catch (e) {}
  for (const entry of opened) {
    if (entry.id) {
      try { await send("touch_opened", { id: entry.id }); } catch (e) {}
    }
  }
  const groupResult = await rebuildGroups(
    createdTabs,
    opened,
    groupHintsOf(rec),
    mode,
    targetGroupId,
  );
  if (groupResult.warning) warnings.push(groupResult.warning);
  // Apply the window state after the tabs exist, so a maximised window does
  // not jump while it is being filled.
  if (win) {
    const state = applyableState(rec && rec.geometry && rec.geometry.state);
    if (state) {
      try {
        await chrome.windows.update(win.id, { state });
      } catch (e) {
        warnings.push("the saved window state could not be restored");
      }
    }
  }
  return {
    count: createdTabs.length,
    grouped: groupResult.grouped,
    warning: warnings.length ? warnings.join("; ") : null,
  };
}

// Public API for popup via runtime messages.
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (!msg || !msg.type) return;
  (async () => {
    try {
      if (msg.type === "startSave") {
        // The popup renders the plan; commits go back through the same drafts.
        const started = await startSaveFor(msg);
        sendResponse({ ok: true, result: started });
      } else if (msg.type === "commitSave") {
        const report = await send("commit_save", { plan: msg.plan });
        sendResponse({ ok: true, result: report });
      } else if (msg.type === "saveCurrentTab") {
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
        const res = await saveTabSet(tabs, { folder: msg.folder, source: "selection" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "saveTabGroup") {
        // The group the active tab belongs to. Asking for a group while the
        // active tab is ungrouped is an error, not a silent whole-window save.
        const active = await chrome.tabs.query({ active: true, currentWindow: true });
        const tab = active && active[0];
        if (!tab) throw new Error("no active tab");
        if (!tab.groupId || tab.groupId === -1) {
          throw new Error("the active tab is not in a tab group");
        }
        const inWindow = await chrome.tabs.query({ windowId: tab.windowId });
        const grouped = inWindow.filter((t) => t.groupId === tab.groupId);
        const res = await saveTabSet(grouped, { folder: msg.folder, source: "group" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "saveWindow") {
        // One window is one set with group annotations, because it reopens as
        // one window; splitting it into a set per group would lose that.
        const active = await chrome.tabs.query({ active: true, currentWindow: true });
        const tab = active && active[0];
        if (!tab) throw new Error("no active window");
        const all = await chrome.tabs.query({ windowId: tab.windowId });
        const res = await saveTabSet(all, { folder: msg.folder, source: "window" });
        sendResponse({ ok: true, result: res });
      } else if (msg.type === "saveWorkspace") {
        // A workspace is the named form of a window capture: same tabs and
        // groups, plus geometry and a stable record that can be updated in
        // place instead of saved again.
        const active = await chrome.tabs.query({ active: true, currentWindow: true });
        const tab = active && active[0];
        if (!tab) throw new Error("no active window");
        const previous = msg.id ? await send("load_record", { id: msg.id }) : null;
        const existing = previous && previous.record;
        const captured = await captureWindow(tab.windowId, {
          folder: msg.folder || (existing && existing.folder) || null,
          existing: existing && Array.isArray(existing.items) ? existing.items : null,
        });
        if (existing) {
          const patch = {
            items: captured.entries,
            tab_groups: captured.groups,
            geometry: captured.geometry,
            saved_from: "window",
          };
          if (typeof msg.title === "string" && msg.title.trim()) {
            patch.title = msg.title.trim();
          }
          if (typeof msg.description === "string") {
            patch.description = msg.description.trim();
          }
          const res = await send("update_record", {
            id: msg.id,
            patch,
            body: captured.body,
          });
          sendResponse({
            ok: true,
            result: {
              id: msg.id,
              count: captured.entries.length,
              groups: captured.groups.length,
              updated: true,
              path: res && res.path,
            },
          });
        } else {
          const title = (msg.title || "").trim() ||
            ("Workspace " + new Date().toLocaleString());
          const record = {
            type: "workspace",
            title,
            tags: [],
            items: captured.entries,
            saved_from: "window",
            geometry: captured.geometry,
          };
          if (msg.description && msg.description.trim()) {
            record.description = msg.description.trim();
          }
          if (captured.groups.length) record.tab_groups = captured.groups;
          if (msg.folder) record.folder = msg.folder;
          const saved = await send("save_record", { record, body: captured.body });
          sendResponse({
            ok: true,
            result: {
              id: saved.id,
              count: captured.entries.length,
              groups: captured.groups.length,
            },
          });
        }
      } else if (msg.type === "listTabGroups") {
        // Live group handles for the "reopen into a group" picker. They are
        // used in this message only and are never written to a record.
        const groups = [];
        if (chrome.tabGroups && chrome.tabGroups.query) {
          const live = await chrome.tabGroups.query({});
          const tabs = await chrome.tabs.query({});
          for (const g of live) {
            groups.push({
              id: g.id,
              title: g.title || "",
              color: g.color || "",
              collapsed: !!g.collapsed,
              windowId: g.windowId,
              tab_count: tabs.filter((t) => t.groupId === g.id).length,
            });
          }
        }
        sendResponse({
          ok: true,
          result: { groups, supported: !!(chrome.tabs.group && chrome.tabGroups) },
        });
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
        const { rec, entries } = await loadSet(msg.id);
        const mode = msg.mode === "window" || msg.mode === "into" ? msg.mode : "group";
        const targetGroupId = typeof msg.groupId === "number" ? msg.groupId : null;
        const result = await openTabSet(rec, entries, { mode, targetGroupId });
        sendResponse({
          ok: true,
          count: result.count,
          mode,
          grouped: result.grouped,
          warning: result.warning,
        });
      } else if (msg.type === "reopenWorkspace") {
        const { rec, entries } = await loadSet(msg.id);
        // A workspace reopens as a window by default: its geometry only means
        // something for a window, and a named workspace is a window.
        const mode = msg.mode === "group" || msg.mode === "into" ? msg.mode : "window";
        const targetGroupId = typeof msg.groupId === "number" ? msg.groupId : null;
        let placement = null;
        let geometryWarning = null;
        if (mode === "window" && rec.geometry) {
          const displays = await displayInfos();
          if (!displays.length) {
            if (
              typeof rec.geometry.left === "number" &&
              typeof rec.geometry.top === "number" &&
              typeof rec.geometry.width === "number" &&
              typeof rec.geometry.height === "number"
            ) {
              placement = rec.geometry;
              geometryWarning =
                "display information is unavailable, so the saved bounds were used without checking them";
            }
          } else {
            try {
              const res = await send("place_window", {
                geometry: rec.geometry,
                displays,
              });
              if (res && res.placement) {
                placement = res.placement;
                if (res.placement.clamped) {
                  geometryWarning =
                    "the window was " +
                    (res.placement.reason || "moved onto a visible display");
                }
              }
            } catch (e) {
              geometryWarning =
                "the saved bounds could not be checked: " +
                (e && e.message ? e.message : e);
            }
          }
        }
        const result = await openTabSet(rec, entries, { mode, targetGroupId, placement });
        if (mode === "window" && !placement) {
          geometryWarning = geometryWarning ||
            (rec.geometry
              ? "the saved bounds were incomplete, so the window opened at the default position"
              : "this workspace has no saved bounds, so the window opened at the default position");
        }
        sendResponse({
          ok: true,
          count: result.count,
          mode,
          grouped: result.grouped,
          warning: [geometryWarning, result.warning].filter(Boolean).join("; ") || null,
          geometry: { applied: !!placement, warning: geometryWarning },
        });
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
    // A keyboard save has no window for a preview, so it commits the same plan
    // the popup would have shown.
    const started = await startSaveFor({ source: "page" });
    if (started && started.plan && started.plan.records.length) {
      await send("commit_save", { plan: started.plan });
    }
  } else if (command === "open-manager") {
    const url = chrome.runtime.getURL("lilac.html");
    await chrome.tabs.create({ url });
  }
});