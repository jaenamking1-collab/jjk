// 팔로우 여부를 + 배지로 짐작하지 않고, Threads 가 받아 오는 데이터의 friendship_status.following 으로 판단한다.
// (답글 화면엔 + 배지가 아예 없어서 배지 방식은 모두를 팔로우로 봤다 — 2026-11-16)
// 페이지 쪽(MAIN world)에서 돌아야 fetch/XHR 응답을 볼 수 있다. 모르는 사람은 링을 달지 않는다.
const follow = new Map(); // username → true/false

function walk(o, depth) {
  if (!o || typeof o !== "object" || depth > 60) return;
  if (Array.isArray(o)) { for (const v of o) walk(v, depth + 1); return; }
  const fs = o.friendship_status;
  if (typeof o.username === "string" && fs && typeof fs.following === "boolean") follow.set(o.username, fs.following);
  for (const k in o) walk(o[k], depth + 1);
}

function eat(text) {
  if (!text || text.indexOf("friendship_status") < 0) return;
  for (const line of text.replace(/^for \(;;\);/, "").split("\n")) {
    try { walk(JSON.parse(line), 0); } catch (e) {}
  }
  schedule();
}

const of = window.fetch;
window.fetch = async function (...a) {
  const r = await of.apply(this, a);
  try { r.clone().text().then(eat, () => {}); } catch (e) {}
  return r;
};
const oo = XMLHttpRequest.prototype.open;
XMLHttpRequest.prototype.open = function (...a) {
  this.addEventListener("load", () => { try { if (this.responseType === "" || this.responseType === "text") eat(this.responseText); } catch (e) {} });
  return oo.apply(this, a);
};

const seen = new WeakSet();
function scanScripts() {
  document.querySelectorAll('script[type="application/json"]').forEach((s) => {
    if (seen.has(s)) return;
    seen.add(s);
    eat(s.textContent);
  });
}

function mark() {
  document.querySelectorAll('img[alt$="프로필 사진"]').forEach((img) => {
    if (img.height < 30 || img.height > 60) return;       // 답글 쌓인 작은 아이콘·큰 프로필 제외
    const a = img.closest('a[href^="/@"]');
    if (!a) return;
    const name = decodeURIComponent(a.getAttribute("href").slice(2).split(/[/?#]/)[0]);
    const f = follow.get(name);
    const box = img.parentElement;                         // 동그란 사진 칸
    box.classList.toggle("tfh-follow", f === true);
    box.dataset.tfh = f === true ? "follow" : f === false ? "no" : "unknown";
  });
}

let t = null;
function schedule() { clearTimeout(t); t = setTimeout(() => { scanScripts(); mark(); }, 200); }

function start() {
  const css = document.createElement("style");
  css.textContent = `
@keyframes tfhPulse {
  0%   { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
  50%  { box-shadow: 0 0 0 5px #ff9500, 0 0 22px 8px rgba(255,200,0,1); transform: scale(1.08); }
  100% { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
}
.tfh-follow { border-radius: 50% !important; animation: tfhPulse 1.4s ease-in-out infinite; }
`;
  document.head.appendChild(css);
  new MutationObserver(schedule).observe(document.body, { childList: true, subtree: true });
  schedule();
}
if (document.body) start(); else document.addEventListener("DOMContentLoaded", start);
window.__tfhFollow = follow; // 콘솔 점검용: __tfhFollow.size
