const $ = (s) => document.querySelector(s);
const list = $("#list");
const status = $("#status");

function setStatus(msg) { status.textContent = msg || ""; }

async function call(type, extra) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(Object.assign({ type }, extra || {}), (resp) => {
      resolve(resp);
    });
  });
}

async function renderRecent() {
  list.innerHTML = "Loading...";
  const resp = await call("listRecent");
  if (!resp || !resp.ok) { list.innerHTML = "Error: " + (resp && resp.error); return; }
  const items = (resp.result && resp.result.results) || [];
  if (!items.length) { list.innerHTML = "<div class='item'>No records yet.</div>"; return; }
  list.innerHTML = items.map((r) => `
    <div class="item">
      <div class="title">${escape(r.title || r.id)}</div>
      <div class="url">${escape(r.id)} · ${escape(r.type)}</div>
      <div class="tags">${(r.tags || []).map(escape).join(", ")}</div>
      <div class="row">
        <button data-open="${r.id}">Open</button>
      </div>
    </div>
  `).join("");
  list.querySelectorAll("[data-open]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await call("reopenRecord", { id: btn.dataset.open });
      setStatus("Opened");
    });
  });
}

function escape(s) { return String(s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c])); }

async function renderCollections() {
  // For v0.1, collections are records with type collection; search by type.
  list.innerHTML = "Loading...";
  const r2 = await call("listRecent");
  const items = ((r2 && r2.result && r2.result.results) || []).filter((x) => x.type === "collection");
  if (!items.length) { list.innerHTML = "<div class='item'>No collections yet.</div>"; return; }
  list.innerHTML = items.map((r) => `
    <div class="item">
      <div class="title">${escape(r.title || r.id)}</div>
      <div class="url">${escape(r.id)}</div>
      <div class="row">
        <button data-reopen="${r.id}">Open all</button>
      </div>
    </div>
  `).join("");
  list.querySelectorAll("[data-reopen]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const res = await call("reopenCollection", { id: btn.dataset.reopen });
      setStatus("Opened " + (res && res.count ? res.count : 0) + " tabs");
    });
  });
}

let currentView = "recent";
async function render() {
  if (currentView === "recent") await renderRecent();
  else await renderCollections();
}

document.querySelectorAll("nav button").forEach((b) => {
  b.addEventListener("click", () => {
    document.querySelectorAll("nav button").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    currentView = b.dataset.view;
    render();
  });
});

$("#saveTab").addEventListener("click", async () => {
  setStatus("Saving...");
  const resp = await call("saveCurrentTab");
  setStatus(resp && resp.ok ? "Saved" : "Error: " + (resp && resp.error));
  render();
});

$("#saveSelected").addEventListener("click", async () => {
  setStatus("Saving selected...");
  const resp = await call("saveSelectedTabs");
  setStatus(resp && resp.ok ? "Saved " + (resp.result && resp.result.count) + " tabs" : "Error: " + (resp && resp.error));
  render();
});

$("#openFolder").addEventListener("click", async () => {
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

render();