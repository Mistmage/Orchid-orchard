const $ = (s) => document.querySelector(s);
const on = (sel, ev, fn) => { const el = $(sel); if (el) el.addEventListener(ev, fn); };
const list = $("#list");
const status = $("#status");
const searchbar = $("#searchbar");
const qInput = $("#q");

function setStatus(msg) { status.textContent = msg || ""; }

async function call(type, extra) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(Object.assign({ type }, extra || {}), (resp) => {
      resolve(resp);
    });
  });
}

function escape(s) { return String(s == null ? "" : s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c])); }

let currentView = "recent";

async function fetchRecent() {
  const resp = await call("listRecent");
  return (resp && resp.ok && resp.result && resp.result.results) || [];
}

async function renderRecent() {
  list.innerHTML = "Loading...";
  const items = await fetchRecent();
  if (!items.length) { list.innerHTML = "<div class='item'>No records yet.</div>"; return; }
  list.innerHTML = items.map((r) => recordCard(r)).join("");
  wireRecordButtons();
}

function recordCard(r) {
  return `
    <div class="item" data-id="${escape(r.id)}">
      <div class="title">${escape(r.title || r.id)}</div>
      <div class="url">${escape(r.id)} · ${escape(r.type)}${r.status ? " · " + escape(r.status) : ""}</div>
      <div class="tags">${(r.tags || []).map(escape).join(", ")}</div>
      <div class="row">
        <button data-open="${escape(r.id)}">Open</button>
        <button data-edit="${escape(r.id)}">Edit</button>
        <button data-del="${escape(r.id)}">Delete</button>
      </div>
    </div>
  `;
}

function wireRecordButtons() {
  list.querySelectorAll("[data-open]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await call("reopenRecord", { id: btn.dataset.open });
      setStatus("Opened");
    });
  });
  list.querySelectorAll("[data-del]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      if (!confirm("Delete this record?")) return;
      await call("deleteRecord", { id: btn.dataset.del });
      setStatus("Deleted");
      render();
    });
  });
  list.querySelectorAll("[data-edit]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await openEditor(btn.dataset.edit);
    });
  });
}

async function openEditor(id) {
  const resp = await call("loadRecord", { id });
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const r = resp.result.record;
  const body = resp.result.body || "";
  const tags = (r.tags || []).join(", ");
  const html = `
    <div class="item">
      <div class="title">Edit: ${escape(r.id)}</div>
      <label>Title<br><input id="e-title" value="${escape(r.title || "")}" style="width:100%" /></label>
      <button id="e-autoname" style="margin:4px 0;">Auto-name</button><br>
      <label>Tags (comma)<br><input id="e-tags" value="${escape(tags)}" style="width:100%" /></label><br>
      <label>Status<br><input id="e-status" value="${escape(r.status || "")}" style="width:100%" /></label><br>
      <label>Notes<br><textarea id="e-body" style="width:100%; height:80px;">${escape(body)}</textarea></label><br>
      <div class="row">
        <button id="e-save">Save</button>
        <button id="e-cancel">Cancel</button>
        <button id="e-applyrules">Apply rules</button>
      </div>
    </div>
  `;
  list.innerHTML = html;
  on("#e-cancel", "click", render);
  const applyBtn = $("#e-applyrules");
  if (applyBtn) applyBtn.addEventListener("click", async () => {
    const record = { id: r.id, type: r.type, title: $("#e-title").value, url: r.url, tags: $("#e-tags").value.split(",").map((s) => s.trim()).filter(Boolean), status: $("#e-status").value || null };
    const resp = await call("applyRules", { record });
    if (resp && resp.ok && resp.result && resp.result.record) {
      const nr = resp.result.record;
      $("#e-title").value = nr.title || "";
      $("#e-tags").value = (nr.tags || []).join(", ");
      $("#e-status").value = nr.status || "";
      $("#e-body").value = resp.result.record.body || $("#e-body").value;
      setStatus("Applied " + (resp.result.fired ? resp.result.fired.length : 0) + " rules");
    } else {
      setStatus("Error: " + (resp && resp.error));
    }
  });
  const autoBtn = $("#e-autoname");
  if (autoBtn) autoBtn.addEventListener("click", async () => {
    const record = { type: r.type, title: r.title, url: r.url, tags: r.tags || [] };
    const rule = { strip: [" - GitHub", " | Hacker News", " - YouTube"], replace: [], prefix: null, suffix: null, max_len: null };
    const resp = await call("deriveTitle", { record, rule });
    if (resp && resp.ok && resp.result && resp.result.title) $("#e-title").value = resp.result.title;
  });
  on("#e-save", "click", async () => {
    const patch = {
      title: $("#e-title").value,
      tags: $("#e-tags").value.split(",").map((s) => s.trim()).filter(Boolean),
      status: $("#e-status").value || null,
    };
    const newBody = $("#e-body").value;
    const r2 = await call("updateRecord", { id, patch, body: newBody });
    if (r2 && r2.ok) { setStatus("Saved"); render(); }
    else setStatus("Error: " + (r2 && r2.error));
  });
}

async function renderCollections() {
  list.innerHTML = "Loading...";
  const all = await fetchRecent();
  const items = all.filter((x) => x.type === "collection");
  if (!items.length) { list.innerHTML = "<div class='item'>No collections yet.</div>"; return; }
  list.innerHTML = items.map((r) => `
    <div class="item">
      <div class="title">${escape(r.title || r.id)}</div>
      <div class="url">${escape(r.id)}</div>
      <div class="row">
        <button data-reopen="${escape(r.id)}">Open all</button>
        <button data-del="${escape(r.id)}">Delete</button>
      </div>
    </div>
  `).join("");
  list.querySelectorAll("[data-reopen]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const res = await call("reopenCollection", { id: btn.dataset.reopen });
      setStatus("Opened " + (res && res.count ? res.count : 0) + " tabs");
    });
  });
  list.querySelectorAll("[data-del]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      if (!confirm("Delete this collection?")) return;
      await call("deleteRecord", { id: btn.dataset.del });
      setStatus("Deleted");
      render();
    });
  });
}

async function renderTags() {
  list.innerHTML = "Loading...";
  const items = await fetchRecent();
  const counts = {};
  for (const r of items) for (const t of (r.tags || [])) counts[t] = (counts[t] || 0) + 1;
  const tags = Object.keys(counts).sort();
  if (!tags.length) { list.innerHTML = "<div class='item'>No tags yet.</div>"; return; }
  list.innerHTML = tags.map((t) => `
    <div class="item">
      <div class="title">${escape(t)} <span style="color:#888">(${counts[t]})</span></div>
      <div class="row"><button data-tag="${escape(t)}">Filter</button></div>
    </div>
  `).join("");
  list.querySelectorAll("[data-tag]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      currentView = "search";
      document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
      document.querySelector('nav button[data-view="search"]').classList.add("active");
      searchbar.style.display = "block";
      qInput.value = "tag:" + btn.dataset.tag;
      await runSearch();
    });
  });
}

async function runSearch() {
  const q = qInput.value.trim();
  list.innerHTML = "Loading...";
  const resp = await call("search", { query: q });
  const items = (resp && resp.ok && resp.result && resp.result.results) || [];
  if (!items.length) { list.innerHTML = "<div class='item'>No results.</div>"; return; }
  list.innerHTML = items.map((r) => recordCard(r)).join("");
  wireRecordButtons();
}

async function renderSearch() {
  searchbar.style.display = "block";
  await runSearch();
}

async function renderSettings() {
  list.innerHTML = "Loading...";
  const cfg = await call("getConfig");
  const rr = (cfg && cfg.result && cfg.result.records_root) || "";
  const preview = await getSetting("previewSingle", false);
  list.innerHTML = `
    <div class="item">
      <div class="title">Settings</div>
      <div class="url">Records root: ${escape(rr)}</div>
      <div class="row">
        <button id="s-open">Open folder</button>
        <button id="s-change">Change folder</button>
      </div>
      <div class="row" style="margin-top:8px;">
        <label><input type="checkbox" id="s-preview" ${preview ? "checked" : ""} /> Preview single record before saving</label>
      </div>
      <div class="row" style="margin-top:8px;">
        <button id="s-rebuild">Rebuild index</button>
        <button id="s-conflicts">Check conflicts</button>
      </div>
      <div class="url" style="margin-top:8px;">Extension ID: <span id="s-extid">loading...</span></div>
    </div>
  `;
  const openBtn = $("#s-open"); if (openBtn) openBtn.addEventListener("click", async () => { await call("openFolder", { which: "records" }); setStatus("Opened"); });
  const changeBtn = $("#s-change"); if (changeBtn) changeBtn.addEventListener("click", async () => {
    const next = prompt("Records folder path:", rr);
    if (!next) return;
    const r = await call("setRecordsRoot", { path: next });
    if (r && r.ok) { setStatus("Records: " + r.result.records_root); renderSettings(); }
    else setStatus("Error: " + (r && r.error));
  });
  const rebuildBtn = $("#s-rebuild"); if (rebuildBtn) rebuildBtn.addEventListener("click", async () => {
    const r = await call("rebuildIndex"); setStatus(r && r.ok ? "Indexed " + r.result.count : "Error");
  });
  const confBtn = $("#s-conflicts"); if (confBtn) confBtn.addEventListener("click", async () => {
    const r = await call("listConflicts"); const c = (r && r.result && r.result.conflicts) || [];
    setStatus(c.length ? "Conflicts: " + c.map((x) => x.path).join(", ") : "No conflicts");
  });
  const previewCb = $("#s-preview"); if (previewCb) previewCb.addEventListener("change", () => setSetting("previewSingle", previewCb.checked));
  // show extension id
  try {
    const id = chrome.runtime.id;
    const el = $("#s-extid"); if (el) el.textContent = id;
  } catch (e) {}
}

async function render() {
  searchbar.style.display = (currentView === "search") ? "block" : "none";
  if (currentView === "recent") await renderRecent();
  else if (currentView === "collections") await renderCollections();
  else if (currentView === "tags") await renderTags();
  else if (currentView === "search") await renderSearch();
  else if (currentView === "settings") await renderSettings();
  if (currentView === "recent") await renderRecent();
  else if (currentView === "collections") await renderCollections();
  else if (currentView === "tags") await renderTags();
  else if (currentView === "search") await renderSearch();
}

document.querySelectorAll("nav button").forEach((b) => {
  b.addEventListener("click", () => {
    document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    currentView = b.dataset.view;
    render();
  });
});

qInput.addEventListener("input", () => { if (currentView === "search") runSearch(); });
qInput.addEventListener("keydown", (e) => { if (e.key === "Enter") runSearch(); });

async function getSetting(key, def) {
  return new Promise((resolve) => {
    chrome.storage.local.get([key], (r) => resolve(r && r[key] != null ? r[key] : def));
  });
}

on("#saveTab", "click", async () => {
  const preview = await getSetting("previewSingle", false);
  if (preview) {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    const t = tabs && tabs[0];
    if (!t) { setStatus("No active tab"); return; }
    let groupInfo = "";
    if (t.groupId && t.groupId !== -1 && chrome.tabGroups && chrome.tabGroups.get) {
      try { const g = await chrome.tabGroups.get(t.groupId); groupInfo = "Group: " + (g.title || "(untitled)") + " (" + g.color + ")"; } catch (e) {}
    }
    const msg = "Save this tab?\n\nTitle: " + (t.title || "") + "\nURL: " + (t.url || "") + (groupInfo ? "\n" + groupInfo : "");
    if (!confirm(msg)) { setStatus("Cancelled"); return; }
  }
  setStatus("Saving...");
  const resp = await call("saveCurrentTab");
  setStatus(resp && resp.ok ? "Saved" : "Error: " + (resp && resp.error));
  render();
});

on("#saveSelected", "click", async () => {
  setStatus("Saving selected...");
  const resp = await call("saveSelectedTabs");
  setStatus(resp && resp.ok ? "Saved " + (resp.result && resp.result.count) + " tabs" : "Error: " + (resp && resp.error));
  render();
});

on("#openFolder", "click", async () => {
  setStatus("Opening folder...");
  const resp = await call("openFolder", { which: "records" });
  if (resp && resp.ok) {
    const cfg = await call("getConfig");
    const p = (cfg && cfg.result && cfg.result.records_root) || (resp.result && resp.result.path);
    setStatus("Records: " + p);
  } else {
    setStatus("Error: " + (resp && resp.error));
  }
});

on("#openManager", "click", async () => {
  const url = chrome.runtime.getURL("lilac.html");
  await chrome.tabs.create({ url });
});

on("#rebuildIndex", "click", async () => {
  setStatus("Rebuilding index...");
  const resp = await call("rebuildIndex");
  setStatus(resp && resp.ok ? "Indexed " + (resp.result && resp.result.count) + " records" : "Error: " + (resp && resp.error));
  render();
});

on("#checkConflicts", "click", async () => {
  setStatus("Checking conflicts...");
  const resp = await call("listConflicts");
  const c = (resp && resp.result && resp.result.conflicts) || [];
  if (!c.length) setStatus("No conflicts found.");
  else setStatus("Conflicts: " + c.map((x) => x.path).join(", "));
});

on("#changeFolder", "click", async () => {
  const cfg = await call("getConfig");
  const current = (cfg && cfg.result && cfg.result.records_root) || "";
  const next = prompt("Records folder path:", current);
  if (!next) return;
  setStatus("Updating folder...");
  const resp = await call("setRecordsRoot", { path: next });
  if (resp && resp.ok) {
    setStatus("Records: " + (resp.result && resp.result.records_root));
  } else {
    setStatus("Error: " + (resp && resp.error));
  }
});

on("#snapshotTab", "click", async () => {
  setStatus("Snapshotting...");
  const resp = await call("snapshotCurrentTab");
  if (resp && resp.ok) {
    setStatus("Snapshotted: " + (resp.result && resp.result.asset && resp.result.asset.path));
    render();
  } else {
    setStatus("Error: " + (resp && resp.error));
  }
});

render();