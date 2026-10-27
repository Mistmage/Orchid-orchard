// Content script: extract page HTML on request (fallback snapshot for non-Chrome).
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.type === "extractHtml") {
    try {
      sendResponse({ ok: true, html: document.documentElement.outerHTML, title: document.title, url: location.href });
    } catch (e) {
      sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
    }
    return true;
  }
});