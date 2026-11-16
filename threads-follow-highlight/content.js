// 팔로우한 사람 프로필 사진에 노란 링. (2026-11-16 v0.3)
// + 배지로는 판단할 수 없다: 답글 화면엔 배지가 없고, 피드에서도 늦게 붙거나 초록 점과 바뀐다. 피드 데이터의 friendship_status 는 null.
// 그래서 화면의 '팔로잉'/'팔로우' 버튼 글자로 사람마다 배워 localStorage 에 기억하고, 팔로잉이 확인된 사람에게만 링을 단다.
// 배우는 곳: 내 팔로잉 목록, 프로필 화면, 마우스를 올리면 뜨는 카드, + 배지(=팔로우 안 함).
const KEY = "tfhFollow";
let follow = {};
try { follow = JSON.parse(localStorage.getItem(KEY) || "{}"); } catch (e) {}
let dirty = false;

const css = document.createElement("style");
css.textContent = `
@keyframes tfhPulse {
  0%   { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
  50%  { box-shadow: 0 0 0 5px #ff9500, 0 0 22px 8px rgba(255,200,0,1); transform: scale(1.08); }
  100% { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
}
.tfh-follow { border-radius: 50% !important; animation: tfhPulse 1.4s ease-in-out infinite; }
#tfh-status { position: fixed; left: 12px; bottom: 12px; z-index: 99999; font: 12px sans-serif;
  background: #ffd400; color: #000; padding: 4px 8px; border-radius: 10px; opacity: .85; pointer-events: none; }
`;
document.documentElement.appendChild(css);

const PROFILE = /^\/@([^/?#]+)\/?$/;
function nameOf(a) { const m = (a.getAttribute("href") || "").match(PROFILE); return m && decodeURIComponent(m[1]); }

function set(name, v) {
  if (!name || follow[name] === v) return;
  follow[name] = v; dirty = true;
}

// 버튼에서 위로 올라가며, 안에 든 프로필 링크가 딱 한 사람이 되는 가장 가까운 칸을 찾는다.
function ownerOf(btn) {
  let el = btn;
  for (let i = 0; i < 10 && el; i++, el = el.parentElement) {
    const names = new Set([...el.querySelectorAll('a[href^="/@"]')].map(nameOf).filter(Boolean));
    if (names.size === 1) return [...names][0];
    if (names.size > 1) return null;                       // 여러 사람이 섞이면 모른다
  }
  const m = location.pathname.match(PROFILE);              // 프로필 화면 위쪽 버튼
  return m && !btn.closest('[role="dialog"]') ? decodeURIComponent(m[1]) : null;
}

function learn() {
  document.querySelectorAll('[role="button"], button').forEach((b) => {
    const t = (b.textContent || "").trim();
    const v = t === "팔로잉" ? true : (t === "팔로우" || t === "맞팔로우") ? false : null;
    if (v === null) return;
    set(ownerOf(b), v);
  });
  // + 배지(aria-label 에 '팔로우')가 붙은 아바타 = 팔로우 안 함
  document.querySelectorAll('[aria-label*="팔로우"]').forEach((b) => {
    const a = b.closest("div") && b.closest("div").parentElement && b.closest("div").parentElement.querySelector('a[href^="/@"]');
    if (a && !(b.textContent || "").trim()) set(nameOf(a), false);
  });
  if (dirty) { try { localStorage.setItem(KEY, JSON.stringify(follow)); } catch (e) {} dirty = false; }
}

function mark() {
  document.querySelectorAll('img[alt$="프로필 사진"]').forEach((img) => {
    if (img.height < 30 || img.height > 60) return;       // 답글 쌓인 작은 아이콘·큰 프로필 제외
    const a = img.closest('a[href^="/@"]');
    const f = a ? follow[nameOf(a)] : undefined;
    const box = img.parentElement;                         // 동그란 사진 칸
    box.classList.toggle("tfh-follow", f === true);
    box.dataset.tfh = f === true ? "follow" : f === false ? "no" : "unknown";
  });
  let s = document.getElementById("tfh-status");
  if (!s) { s = document.createElement("div"); s.id = "tfh-status"; document.body.appendChild(s); }
  const n = Object.values(follow).filter((v) => v === true).length;
  s.textContent = n < 5 ? `노란 링: 팔로잉 ${n}명 기억 · 내 프로필 > 팔로잉 목록을 한 번 끝까지 내려 주세요` : `노란 링: 팔로잉 ${n}명 기억`;
}

let t = null;
new MutationObserver(() => { clearTimeout(t); t = setTimeout(() => { learn(); mark(); }, 200); })
  .observe(document.body, { childList: true, subtree: true, characterData: true });
learn(); mark();
