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

function escape(s) {
  return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}

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
      <div class="url">${escape(r.id)} · ${escape(r.type)}${r.status ? " · " + escape(r.status) : ""}${r.folder ? " · 📁" + escape(r.folder) : ""}</div>
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
      <label>Description<br><input id="e-description" value="${escape(r.description || "")}" style="width:100%" /></label><br>
      <label>Folder (relative to Records/)<br><input id="e-folder" value="${escape(r.folder || "")}" style="width:100%" /></label><br>
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
      setStatus(
        "Applied " +
          (resp.result.fired ? resp.result.fired.length : 0) +
          " rules" +
          (resp.result.errors && resp.result.errors.length
            ? " · " + resp.result.errors.join("; ")
            : ""),
      );
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
      description: $("#e-description").value || null,
      folder: $("#e-folder").value || null,
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
  list.innerHTML = items.map((r) => {
    const count = r.item_count || 0;
    const source = r.saved_from ? escape(r.saved_from) + " · " : "";
    const group = r.group_name ? " · " + escape(r.group_name) + (r.group_color ? " (" + escape(r.group_color) + ")" : "") : "";
    return `
    <div class="item">
      <div class="title">${escape(r.title || r.id)}</div>
      <div class="url">${source}${count} tab${count === 1 ? "" : "s"}${group} · ${escape(r.id)}</div>
      <div class="row">
        <select data-mode="${escape(r.id)}" title="How to reopen this set">
          <option value="group">New group</option>
          <option value="window">New window</option>
          <option value="into">Into group…</option>
        </select>
        <select data-target="${escape(r.id)}" style="display:none" title="Target group"></select>
        <button data-reopen="${escape(r.id)}">Open</button>
        <button data-edit="${escape(r.id)}">Edit</button>
        <button data-del="${escape(r.id)}">Delete</button>
      </div>
    </div>`;
  }).join("");
  // Live groups are only used to fill the picker; the ids stay in this page.
  let liveGroups = null;
  async function ensureGroups() {
    if (liveGroups) return liveGroups;
    const resp = await call("listTabGroups");
    liveGroups = (resp && resp.ok && resp.result && resp.result.groups) || [];
    return liveGroups;
  }
  list.querySelectorAll("select[data-mode]").forEach((select) => {
    const id = select.dataset.mode;
    const target = list.querySelector(`select[data-target="${id}"]`);
    select.addEventListener("change", async () => {
      if (select.value !== "into") { target.style.display = "none"; return; }
      const groups = await ensureGroups();
      if (!groups.length) {
        setStatus("No open tab groups to add to");
        select.value = "group";
        target.style.display = "none";
        return;
      }
      target.innerHTML = groups.map((g) => {
        const title = g.title || "(untitled)";
        return `<option value="${escape(g.id)}">${escape(title)} · ${escape(g.color)} · ${g.tab_count} tab${g.tab_count === 1 ? "" : "s"}</option>`;
      }).join("");
      target.style.display = "";
    });
  });
  list.querySelectorAll("[data-reopen]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.reopen;
      const modeSelect = list.querySelector(`select[data-mode="${id}"]`);
      const targetSelect = list.querySelector(`select[data-target="${id}"]`);
      const mode = modeSelect ? modeSelect.value : "group";
      const groupId = mode === "into" && targetSelect && targetSelect.value
        ? Number(targetSelect.value)
        : null;
      setStatus("Opening...");
      const res = await call("reopenCollection", { id, mode, groupId });
      if (!res || !res.ok) { setStatus("Error: " + (res && res.error)); return; }
      const opened = (res.count || 0) + " tab" + ((res.count || 0) === 1 ? "" : "s");
      setStatus(res.warning ? "Opened " + opened + " — " + res.warning : "Opened " + opened);
    });
  });
  list.querySelectorAll("[data-edit]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await openEditor(btn.dataset.edit);
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

function geometryText(g) {
  if (!g || typeof g.width !== "number" || typeof g.height !== "number") return "";
  const size = Math.round(g.width) + "×" + Math.round(g.height);
  const at = typeof g.left === "number" && typeof g.top === "number"
    ? " at " + Math.round(g.left) + "," + Math.round(g.top)
    : "";
  const state = g.state ? " · " + g.state : "";
  return size + at + state;
}

async function fetchLiveGroups() {
  const resp = await call("listTabGroups");
  return (resp && resp.ok && resp.result && resp.result.groups) || [];
}

async function renderWorkspaces() {
  list.innerHTML = "Loading...";
  const all = await fetchRecent();
  const items = all.filter((x) => x.type === "workspace");
  list.innerHTML = `
    <div class="item">
      <div class="title">Save this window as a workspace</div>
      <label>Name<br><input id="w-title" placeholder="e.g. Design desk" style="width:100%" /></label>
      <label>Description<br><input id="w-desc" placeholder="What this arrangement is for" style="width:100%" /></label>
      <div class="row"><button id="w-save">Save current window</button></div>
      <div class="url">Tabs, groups and window bounds are captured. Reopening clamps the bounds to a display that exists now.</div>
    </div>
    ${items.map((r) => {
      const count = r.item_count || 0;
      const geometry = geometryText(r.geometry);
      return `
      <div class="item">
        <div class="title">${escape(r.title || r.id)}</div>
        <div class="url">${count} tab${count === 1 ? "" : "s"}${geometry ? " · " + escape(geometry) : ""} · ${escape(r.id)}</div>
        <div class="row">
          <select data-wmode="${escape(r.id)}" title="How to reopen this workspace">
            <option value="window">New window</option>
            <option value="group">New group</option>
            <option value="into">Into group…</option>
          </select>
          <select data-wtarget="${escape(r.id)}" style="display:none" title="Target group"></select>
          <button data-wopen="${escape(r.id)}">Open</button>
          <button data-wupdate="${escape(r.id)}" title="Replace this workspace with the current window">Update</button>
          <button data-wedit="${escape(r.id)}">Edit</button>
          <button data-wdel="${escape(r.id)}">Delete</button>
        </div>
      </div>`;
    }).join("")}
  `;
  const saveBtn = $("#w-save");
  if (saveBtn) saveBtn.addEventListener("click", async () => {
    const title = $("#w-title").value.trim();
    const description = $("#w-desc").value.trim();
    setStatus("Saving workspace...");
    const resp = await call("saveWorkspace", { title, description });
    if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
    const res = resp.result || {};
    setStatus("Saved " + (res.count || 0) + " tabs as a workspace");
    renderWorkspaces();
  });
  // Live groups fill the "into" picker; the ids never leave this page.
  list.querySelectorAll("select[data-wmode]").forEach((select) => {
    const id = select.dataset.wmode;
    const target = list.querySelector(`select[data-wtarget="${id}"]`);
    select.addEventListener("change", async () => {
      if (select.value !== "into") { target.style.display = "none"; return; }
      const groups = await fetchLiveGroups();
      if (!groups.length) {
        setStatus("No open tab groups to add to");
        select.value = "window";
        target.style.display = "none";
        return;
      }
      target.innerHTML = groups.map((g) => {
        const title = g.title || "(untitled)";
        return `<option value="${escape(g.id)}">${escape(title)} · ${escape(g.color)} · ${g.tab_count} tab${g.tab_count === 1 ? "" : "s"}</option>`;
      }).join("");
      target.style.display = "";
    });
  });
  list.querySelectorAll("[data-wopen]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.wopen;
      const modeSelect = list.querySelector(`select[data-wmode="${id}"]`);
      const targetSelect = list.querySelector(`select[data-wtarget="${id}"]`);
      const mode = modeSelect ? modeSelect.value : "window";
      const groupId = mode === "into" && targetSelect && targetSelect.value
        ? Number(targetSelect.value)
        : null;
      setStatus("Opening...");
      const res = await call("reopenWorkspace", { id, mode, groupId });
      if (!res || !res.ok) { setStatus("Error: " + (res && res.error)); return; }
      const opened = (res.count || 0) + " tab" + ((res.count || 0) === 1 ? "" : "s");
      setStatus(res.warning ? "Opened " + opened + " — " + res.warning : "Opened " + opened);
    });
  });
  list.querySelectorAll("[data-wupdate]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.wupdate;
      if (!confirm("Update this workspace from the current window?")) return;
      setStatus("Updating workspace...");
      const resp = await call("saveWorkspace", { id });
      if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
      const res = resp.result || {};
      setStatus("Updated " + (res.count || 0) + " tabs");
      renderWorkspaces();
    });
  });
  list.querySelectorAll("[data-wedit]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await openEditor(btn.dataset.wedit);
    });
  });
  list.querySelectorAll("[data-wdel]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      if (!confirm("Delete this workspace?")) return;
      await call("deleteRecord", { id: btn.dataset.wdel });
      setStatus("Deleted");
      renderWorkspaces();
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
  const defaultFolder = await getSetting("defaultSaveFolder", "");
  const registered = (cfg && cfg.result && cfg.result.registered_folders) || [];
  list.innerHTML = `
    <div class="item">
      <div class="title">Settings</div>
      <div class="url">Records root: ${escape(rr)}</div>
      <div class="url">Registered folders: ${registered.length ? registered.map(escape).join(", ") : "(none — using Records/<type>)"}</div>
      <div class="url" style="opacity:0.7;">Managed in Orchard Settings (Orchid owns the folder list).</div>
      <div class="row" style="margin-top:6px;">
        <label>Default save folder:
          <select id="s-defaultfolder" style="margin-left:6px;">
            <option value="">(auto: Records/<type>)</option>
            ${registered.map((f) => `<option value="${escape(f)}" ${f === defaultFolder ? "selected" : ""}>${escape(f)}</option>`).join("")}
          </select>
        </label>
      </div>
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
  const defSel = $("#s-defaultfolder"); if (defSel) defSel.addEventListener("change", () => setSetting("defaultSaveFolder", defSel.value));
  // Naming is configured in Orchard Settings; the host owns the config.
  list.innerHTML += `
    <div class="item" style="margin-top:8px;">
      <div class="title">Naming rules</div>
      <div class="url">Managed in Orchard Settings → Naming (global + per-domain templates).</div>
      <div class="row" style="margin-top:6px;">
        <button id="s-apply-rule">Auto-name all</button>
      </div>
    </div>
  `;
  const applyRuleBtn = $("#s-apply-rule"); if (applyRuleBtn) applyRuleBtn.addEventListener("click", async () => {
    setStatus("Applying naming rules...");
    const items = await fetchRecent();
    let n = 0;
    for (const r of items) {
      const resp = await call("deriveTitle", { record: { id: r.id, type: r.type, title: r.title, url: r.url, tags: r.tags || [] } });
      if (resp && resp.ok && resp.result && resp.result.title) {
        await call("updateRecord", { id: r.id, patch: { title: resp.result.title } });
        n++;
      }
    }
    setStatus("Renamed " + n + " records");
    render();
  });
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
  else if (currentView === "workspaces") await renderWorkspaces();
  else if (currentView === "tags") await renderTags();
  else if (currentView === "search") await renderSearch();
  else if (currentView === "settings") await renderSettings();
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
  const folder = await getSetting("defaultSaveFolder", "");
  const resp = await call("saveCurrentTab", { folder: folder || undefined });
  setStatus(resp && resp.ok ? "Saved" : "Error: " + (resp && resp.error));
  render();
});

on("#saveSelected", "click", async () => {
  setStatus("Saving selected...");
  const folder = await getSetting("defaultSaveFolder", "");
  const resp = await call("saveSelectedTabs", { folder: folder || undefined });
  setStatus(resp && resp.ok ? "Saved " + (resp.result && resp.result.count) + " tabs" : "Error: " + (resp && resp.error));
  render();
});

on("#saveGroup", "click", async () => {
  setStatus("Saving group...");
  const folder = await getSetting("defaultSaveFolder", "");
  const resp = await call("saveTabGroup", { folder: folder || undefined });
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const groups = (resp.result && resp.result.groups) || 0;
  setStatus("Saved " + (resp.result && resp.result.count) + " tabs" + (groups ? " in " + groups + " group" + (groups === 1 ? "" : "s") : ""));
  render();
});

on("#saveWindow", "click", async () => {
  setStatus("Saving window...");
  const folder = await getSetting("defaultSaveFolder", "");
  const resp = await call("saveWindow", { folder: folder || undefined });
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const groups = (resp.result && resp.result.groups) || 0;
  setStatus("Saved " + (resp.result && resp.result.count) + " tabs" + (groups ? " in " + groups + " group" + (groups === 1 ? "" : "s") : ""));
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