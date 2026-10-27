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
      <label>Title<br><input id="e-title" value="${escape(r.title || "")}" style="width:100%" /></label><br>
      <label>Tags (comma)<br><input id="e-tags" value="${escape(tags)}" style="width:100%" /></label><br>
      <label>Status<br><input id="e-status" value="${escape(r.status || "")}" style="width:100%" /></label><br>
      <label>Notes<br><textarea id="e-body" style="width:100%; height:80px;">${escape(body)}</textarea></label><br>
      <div class="row">
        <button id="e-save">Save</button>
        <button id="e-cancel">Cancel</button>
      </div>
    </div>
  `;
  list.innerHTML = html;
  on("#e-cancel", "click", render);
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

async function render() {
  searchbar.style.display = (currentView === "search") ? "block" : "none";
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

on("#saveTab", "click", async () => {
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