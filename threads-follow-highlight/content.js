// 팔로우 안 한 사람 아바타에는 + 배지가 붙는다. 배지가 없는 아바타 = 팔로우한 사람 → 노란 링이 깜빡이며 커졌다 작아진다.
// 아바타 칸: img → div → a → div → div(칸). 칸 안에 + 배지가 있으면 팔로우 안 한 사람.
// 초록 점(활동 중 표시)은 배지가 아니다 — 있어도 팔로우한 사람으로 본다.
const css = document.createElement("style");
css.textContent = `
@keyframes tfhPulse {
  0%   { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
  50%  { box-shadow: 0 0 0 5px #ff9500, 0 0 22px 8px rgba(255,200,0,1); transform: scale(1.08); }
  100% { box-shadow: 0 0 0 3px #ffd400, 0 0 6px 2px rgba(255,212,0,.7); transform: scale(1); }
}
.tfh-follow { border-radius: 50% !important; animation: tfhPulse 1.4s ease-in-out infinite; }
`;
document.documentElement.appendChild(css);

function isGreen(el) {
  const c = getComputedStyle(el).backgroundColor.match(/\d+/g);
  if (!c) return false;
  const [r, g, b] = c.map(Number);
  return g > 140 && g > r + 40 && g > b + 40;
}

function isPlusBadge(el) {
  if ((el.getAttribute("aria-label") || "").includes("팔로우")) return true;
  const svg = el.tagName.toLowerCase() === "svg" ? el : el.querySelector("svg");
  if (!svg) return false;                                  // svg 없는 것(초록 점 등)은 배지 아님
  if (!svg.querySelector("path, line, polygon, rect")) return false; // 동그라미만 있는 점 제외
  return !isGreen(el);
}

function mark() {
  document.querySelectorAll('img[alt$="프로필 사진"]').forEach((img) => {
    if (img.height < 30 || img.height > 60) return;       // 답글 쌓인 작은 아이콘·큰 프로필 제외
    const a = img.closest("a");
    const cell = a && a.parentElement && a.parentElement.parentElement;
    if (!cell) return;
    const box = img.parentElement;                         // 동그란 사진 칸
    const hasPlus = [...cell.querySelectorAll('svg, [role="button"], [aria-label*="팔로우"]')].some(isPlusBadge);
    box.classList.toggle("tfh-follow", !hasPlus);
    box.dataset.tfh = hasPlus ? "no" : "follow";
  });
}

let t = null;
new MutationObserver(() => { clearTimeout(t); t = setTimeout(mark, 200); })
  .observe(document.body, { childList: true, subtree: true });
mark();
