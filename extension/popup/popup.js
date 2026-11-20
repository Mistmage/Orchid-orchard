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
    await openSavePreview({ source: "workspace", title, description });
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

// ---------------------------------------------------------------------------
// Save preview
//
// A save is planned in Rust and rendered here. The same drafts are committed,
// so what the preview shows is what gets written; edits mutate the drafts.
// ---------------------------------------------------------------------------

let pendingSave = null;

const KNOWN_RECORD_KEYS = new Set([
  "id", "type", "target_type", "version", "title", "icon", "color",
  "description", "created", "modified", "opened", "last_viewed", "parent",
  "related", "links", "tags", "categories", "status", "url", "folder",
  "items", "saved_from", "tab_groups", "geometry", "authority", "lineage",
  "forked_from", "snapshot", "snapshot_format",
]);

function draftTypeLabel(record) {
  return record.type + (record.target_type ? " · " + record.target_type : "");
}

function draftWarnings(draft) {
  const warnings = (draft.warnings || []).slice();
  if (draft.duplicate_of) warnings.push("duplicate of " + draft.duplicate_of);
  return warnings;
}

function extraFields(record) {
  return Object.keys(record).filter((key) => !KNOWN_RECORD_KEYS.has(key));
}

function primitiveInput(key, value, extra) {
  const attr = extra ? ` data-extra="${escape(key)}"` : "";
  const idAttr = extra ? "" : ` id="p-${escape(key)}"`;
  if (typeof value === "boolean") {
    return `<label>${escape(key)} <input type="checkbox"${idAttr}${attr} ${value ? "checked" : ""} /></label>`;
  }
  const type = typeof value === "number" ? "number" : "text";
  return `<label>${escape(key)} <input${idAttr} type="${type}"${attr} value="${escape(value)}" style="width:100%" /></label>`;
}

function recordDetail(draft, index, showBack) {
  const r = draft.record;
  const warnings = draftWarnings(draft);
  const extra = extraFields(r).map((key) => {
    const value = r[key];
    if (value == null) return "";
    if (typeof value === "object") {
      return `<div class="url">${escape(key)}: <code class="mono">${escape(JSON.stringify(value))}</code></div>`;
    }
    return `<div class="row">${primitiveInput(key, value, true)}</div>`;
  }).join("");
  return `
    <div class="item">
      <div class="title">${escape(r.title || "(untitled)")}</div>
      <div class="url">${escape(draftTypeLabel(r))} · ${escape(draft.path)}</div>
      <div class="row"><label>Title <input id="p-title" value="${escape(r.title || "")}" style="width:100%" /></label></div>
      <div class="row"><label>Folder (under Records/; blank = type folder) <input id="p-folder" value="${escape(r.folder || "")}" style="width:100%" /></label></div>
      <div class="row"><label>Tags (comma separated) <input id="p-tags" value="${escape((r.tags || []).join(", "))}" style="width:100%" /></label></div>
      <div class="row"><label>Status <input id="p-status" value="${escape(r.status || "")}" style="width:100%" /></label></div>
      <div class="row"><label>Description <input id="p-description" value="${escape(r.description || "")}" style="width:100%" /></label></div>
      ${r.url != null ? `<div class="row"><label>URL <input id="p-url" value="${escape(r.url)}" style="width:100%" /></label></div>` : ""}
      <div class="url">id ${escape(r.id)} · ${escape(r.type)}${r.saved_from ? " · saved from " + escape(r.saved_from) : ""}</div>
      <div class="url">file: <span id="p-path">${escape(draft.path)}</span></div>
      <div class="url">filename: <span id="p-filename">${escape(draft.filename || "")}</span></div>
      ${r.snapshot ? `<div class="url">snapshot: ${escape(r.snapshot)}${r.snapshot_format ? " (" + escape(r.snapshot_format) + ")" : ""}</div>` : ""}
      ${warnings.length ? `<div class="warnings">${warnings.map((w) => `<div class="error">${escape(w)}</div>`).join("")}</div>` : ""}
      ${extra}
      <div class="row"><label>Body<br><textarea id="p-body" rows="6" style="width:100%">${escape(draft.body || "")}</textarea></label></div>
      <div class="row">
        <label><input type="checkbox" id="p-excluded" ${draft.excluded ? "checked" : ""} /> Exclude this record</label>
      </div>
      <div class="row">
        ${showBack ? `<button id="p-back">Back to list</button>` : ""}
        <button id="p-commit-one">Save ${showBack ? "all included" : "record"}</button>
      </div>
    </div>`;
}

function collectDetail(draft) {
  const r = draft.record;
  const read = (id) => { const el = $(id); return el ? el.value : undefined; };
  const title = read("#p-title");
  if (title !== undefined) r.title = title.trim() ? title : null;
  const folder = read("#p-folder");
  if (folder !== undefined) r.folder = folder.trim() ? folder.trim() : null;
  const tags = read("#p-tags");
  if (tags !== undefined) r.tags = tags.split(",").map((s) => s.trim()).filter(Boolean);
  const status = read("#p-status");
  if (status !== undefined) r.status = status.trim() ? status : null;
  const description = read("#p-description");
  if (description !== undefined) r.description = description.trim() ? description : null;
  const url = read("#p-url");
  if (url !== undefined) r.url = url.trim() ? url : null;
  const body = read("#p-body");
  if (body !== undefined) draft.body = body;
  const excluded = $("#p-excluded");
  if (excluded) draft.excluded = !!excluded.checked;
  list.querySelectorAll("[data-extra]").forEach((el) => {
    const key = el.dataset.extra;
    if (!key) return;
    if (el.type === "checkbox") r[key] = !!el.checked;
    else if (el.type === "number") r[key] = el.value === "" ? null : Number(el.value);
    else r[key] = el.value === "" ? null : el.value;
  });
}

async function refreshDraftPath(draft) {
  try {
    const resp = await call("planPath", { record: draft.record });
    if (resp && resp.ok && resp.result) {
      draft.path = resp.result.path;
      draft.filename = resp.result.filename;
      const el = $("#p-path");
      if (el) el.textContent = draft.path;
      const name = $("#p-filename");
      if (name) name.textContent = draft.filename;
    }
  } catch (e) {}
}

function renderPreview() {
  const plan = pendingSave && pendingSave.plan;
  if (!plan) { list.innerHTML = "<div class='item'>No plan.</div>"; return; }
  const records = plan.records || [];
  const single = records.length === 1;
  if (single && pendingSave.detail == null) pendingSave.detail = 0;
  const detailIndex = pendingSave.detail;

  if (detailIndex != null && records[detailIndex]) {
    list.innerHTML = `
      ${single ? "" : `<div class="row"><button id="p-back-top">Back to list</button></div>`}
      ${recordDetail(records[detailIndex], detailIndex, !single)}`;
    const back = $("#p-back"); if (back) back.addEventListener("click", async () => { collectDetail(records[detailIndex]); await refreshDraftPath(records[detailIndex]); pendingSave.detail = null; renderPreview(); });
    const backTop = $("#p-back-top"); if (backTop) backTop.addEventListener("click", async () => { collectDetail(records[detailIndex]); await refreshDraftPath(records[detailIndex]); pendingSave.detail = null; renderPreview(); });
    const commitOne = $("#p-commit-one"); if (commitOne) commitOne.addEventListener("click", commitPreview);
    ["#p-title", "#p-folder", "#p-url"].forEach((sel) => {
      const el = $(sel);
      if (el) el.addEventListener("change", () => { collectDetail(records[detailIndex]); refreshDraftPath(records[detailIndex]); });
    });
    return;
  }

  const rows = records.map((draft, index) => {
    const warnings = draftWarnings(draft);
    return `
      <div class="item">
        <div class="row">
          <label><input type="checkbox" data-exclude="${index}" ${draft.excluded ? "" : "checked"} /> include</label>
          <div class="title" style="flex:1">${escape(draft.record.title || "(untitled)")}</div>
        </div>
        <div class="url">${escape(draftTypeLabel(draft.record))} · ${escape(draft.path)}</div>
        ${warnings.length ? `<div class="warnings">${warnings.map((w) => `<div class="error">${escape(w)}</div>`).join("")}</div>` : ""}
        <div class="row">
          <button data-detail="${index}">Details</button>
        </div>
      </div>`;
  }).join("");

  list.innerHTML = `
    <div class="item">
      <div class="title">Preview · ${escape(plan.source || "")}</div>
      <div class="url">${records.length} record${records.length === 1 ? "" : "s"}; ${records.filter((d) => !d.excluded).length} will be written.</div>
      ${(plan.duplicates || []).length ? `<div class="url">${plan.duplicates.length} duplicate page${plan.duplicates.length === 1 ? "" : "s"} already saved (reported, not dropped yet).</div>` : ""}
      ${(plan.warnings || []).length ? `<div class="warnings">${plan.warnings.map((w) => `<div class="error">${escape(w)}</div>`).join("")}</div>` : ""}
      <div class="row">
        <button id="p-commit">Save ${records.filter((d) => !d.excluded).length}</button>
        <button id="p-cancel">Cancel</button>
      </div>
    </div>
    ${rows}`;

  list.querySelectorAll("[data-exclude]").forEach((cb) => {
    cb.addEventListener("change", () => {
      const draft = records[Number(cb.dataset.exclude)];
      if (draft) draft.excluded = !cb.checked;
      const commit = $("#p-commit");
      if (commit) commit.textContent = "Save " + records.filter((d) => !d.excluded).length;
    });
  });
  list.querySelectorAll("[data-detail]").forEach((btn) => {
    btn.addEventListener("click", () => {
      pendingSave.detail = Number(btn.dataset.detail);
      renderPreview();
    });
  });
  const commit = $("#p-commit"); if (commit) commit.addEventListener("click", commitPreview);
  const cancel = $("#p-cancel"); if (cancel) cancel.addEventListener("click", cancelPreview);
}

async function openSavePreview(msg) {
  setStatus("Planning...");
  const resp = await call("startSave", msg);
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const result = resp.result || {};
  pendingSave = { plan: result.plan, source: result.source, detail: null };
  currentView = "preview";
  document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
  setStatus("");
  renderPreview();
}

function cancelPreview() {
  const plan = pendingSave && pendingSave.plan;
  const counted = plan && plan.records ? plan.records.length : 0;
  pendingSave = null;
  currentView = "recent";
  document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
  const recent = document.querySelector('nav button[data-view="recent"]');
  if (recent) recent.classList.add("active");
  setStatus(counted ? "Cancelled; nothing was saved" : "Cancelled");
  render();
}

async function commitPreview() {
  if (!pendingSave) return;
  if (pendingSave.detail != null) {
    const draft = pendingSave.plan.records[pendingSave.detail];
    if (draft) collectDetail(draft);
  }
  setStatus("Saving...");
  const resp = await call("commitSave", { plan: pendingSave.plan });
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const report = resp.result || {};
  const written = (report.written || []).length;
  const skipped = (report.skipped || []).length;
  const failed = (report.failed || []).length;
  const parts = ["Saved " + written];
  if (skipped) parts.push(skipped + " skipped");
  if (failed) parts.push(failed + " failed: " + (report.failed || []).map((f) => f.id + " (" + f.error + ")").join("; "));
  pendingSave = null;
  currentView = "recent";
  document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
  const recent = document.querySelector('nav button[data-view="recent"]');
  if (recent) recent.classList.add("active");
  setStatus(parts.join(" · "));
  render();
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
  const instant = await getSetting("instantSave", false);
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
        <label><input type="checkbox" id="s-instant" ${instant ? "checked" : ""} /> Save page instantly (skip the preview)</label>
      </div>
      <div class="url" style="opacity:0.7;">Off by default. Only single-page saves skip the preview; selections, groups, windows, workspaces and snapshots are always shown first. Rules, de-duplication and placement still run.</div>
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
  const instantCb = $("#s-instant"); if (instantCb) instantCb.addEventListener("change", () => setSetting("instantSave", instantCb.checked));
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
  else if (currentView === "preview") renderPreview();
  else if (currentView === "tags") await renderTags();
  else if (currentView === "search") await renderSearch();
  else if (currentView === "settings") await renderSettings();
}

document.querySelectorAll("nav button").forEach((b) => {
  b.addEventListener("click", () => {
    // Leaving a preview abandons the plan: nothing has been written.
    pendingSave = null;
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

async function saveInstantly(msg) {
  setStatus("Saving...");
  const resp = await call("startSave", msg);
  if (!resp || !resp.ok) { setStatus("Error: " + (resp && resp.error)); return; }
  const committed = await call("commitSave", { plan: resp.result.plan });
  if (!committed || !committed.ok) { setStatus("Error: " + (committed && committed.error)); return; }
  const report = committed.result || {};
  const failed = (report.failed || []).length;
  setStatus("Saved " + (report.written || []).length + (failed ? " · " + failed + " failed" : ""));
  render();
}

on("#saveTab", "click", async () => {
  const folder = await getSetting("defaultSaveFolder", "");
  const instant = await getSetting("instantSave", false);
  if (instant) { await saveInstantly({ source: "page", folder: folder || undefined }); return; }
  await openSavePreview({ source: "page", folder: folder || undefined });
});

on("#saveSelected", "click", async () => {
  const folder = await getSetting("defaultSaveFolder", "");
  await openSavePreview({ source: "selection", folder: folder || undefined });
});

on("#saveGroup", "click", async () => {
  const folder = await getSetting("defaultSaveFolder", "");
  await openSavePreview({ source: "group", folder: folder || undefined });
});

on("#saveWindow", "click", async () => {
  const folder = await getSetting("defaultSaveFolder", "");
  await openSavePreview({ source: "window", folder: folder || undefined });
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
  const folder = await getSetting("defaultSaveFolder", "");
  await openSavePreview({ source: "snapshot", folder: folder || undefined });
});

render();