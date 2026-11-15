// 팔로우 안 한 사람 아바타에는 + 배지가 붙는다. 배지가 없는 아바타 = 팔로우한 사람 → 노란 링.
// 아바타 칸: img → div → a → div → div(칸). 칸 안에 svg / role=button / '팔로우' 라벨이 있으면 배지로 본다.
const RING = "0 0 0 3px #ffd400, 0 0 10px 3px rgba(255,212,0,.85)";

function mark() {
  document.querySelectorAll('img[alt$="프로필 사진"]').forEach((img) => {
    if (img.height < 30 || img.height > 60) return;       // 답글 쌓인 작은 아이콘·큰 프로필 제외
    const a = img.closest("a");
    const cell = a && a.parentElement && a.parentElement.parentElement;
    if (!cell) return;
    const box = img.parentElement;                         // 동그란 사진 칸
    const hasPlus = !!cell.querySelector('svg, [role="button"], [aria-label*="팔로우"]');
    box.style.borderRadius = "50%";
    box.style.boxShadow = hasPlus ? "" : RING;
    box.dataset.tfh = hasPlus ? "no" : "follow";
  });
}

let t = null;
new MutationObserver(() => { clearTimeout(t); t = setTimeout(mark, 200); })
  .observe(document.body, { childList: true, subtree: true });
mark();
