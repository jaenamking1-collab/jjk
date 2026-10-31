// 페이지 안: Shift+클릭 = 한 개, Shift+드래그 = 페이지 전체
(() => {
  if (window.__jupjupAlive && window.__jupjupAlive()) return;
  const alive = () => !!(chrome.runtime && chrome.runtime.id);
  window.__jupjupAlive = alive;

  const DRAG_PX = 40;
  const MIN_IMG_PX = 100; // 전체 줍기에서 이보다 작은 그림(아이콘 등)은 뺀다
  let start = null, dragging = false, swallowClick = false, hint = null;

  const httpUrl = u => {
    try {
      const url = new URL(u, location.href);
      return /^https?:$/.test(url.protocol) ? url.href : '';
    } catch (e) { return ''; }
  };

  const isEditable = el => el instanceof Element &&
    (el.isContentEditable || el.closest('input,textarea,select,[contenteditable=""],[contenteditable="true"]'));

  function imgUrl(img) {
    return httpUrl(img.currentSrc || img.src) || httpUrl(img.dataset.src || img.getAttribute('data-original') || '');
  }

  function videoUrl(v) {
    const direct = v.currentSrc || v.src || (v.querySelector('source') || {}).src || '';
    // 유튜브처럼 blob: 으로 재생되는 영상은 주소가 없어 그 페이지 주소를 쓴다
    return httpUrl(direct) || location.href;
  }

  function bgUrl(el) {
    const m = /url\(["']?([^"')]+)/.exec(getComputedStyle(el).backgroundImage || '');
    return m ? httpUrl(m[1]) : '';
  }

  function pick(x, y) {
    const stack = document.elementsFromPoint(x, y);
    const top = stack[0];
    if (!top) return null;
    // 글자 바로 위를 누른 경우만 링크다. 사진+제목이 한 링크인 카드에서는 사진을 고른다
    const onText = [...top.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
    const textLink = onText && top.closest('a[href]');
    if (textLink) {
      const url = httpUrl(textLink.href);
      if (url) return { url, kind: 'link' };
    }
    // 그림 위에 투명한 막이 덮인 사이트가 많아 아래쪽까지 본다
    for (const el of stack.slice(0, 6)) {
      if (el.tagName === 'VIDEO') return { url: videoUrl(el), kind: 'video' };
      if (el.tagName === 'IMG') {
        const url = imgUrl(el);
        if (url) return { url, kind: 'image' };
      }
    }
    const link = top.closest('a[href]');
    if (link) {
      const url = httpUrl(link.href);
      if (url) return { url, kind: 'link' };
    }
    for (const el of stack.slice(0, 3)) {
      if (el === document.documentElement || el === document.body) break;
      const url = bgUrl(el);
      if (url) return { url, kind: 'image' };
    }
    return null;
  }

  function walk(root, out) {
    for (const el of root.querySelectorAll('*')) {
      if (el.shadowRoot) walk(el.shadowRoot, out);
      out.push(el);
    }
    return out;
  }

  function collectAll() {
    const seen = new Set(), items = [];
    const add = (url, kind) => {
      if (url && !seen.has(url)) { seen.add(url); items.push({ url, kind }); }
    };
    const here = location.href.split('#')[0];
    for (const el of walk(document, [])) {
      if (el.tagName === 'A' && el.href) {
        const url = httpUrl(el.href);
        if (url && url.split('#')[0] !== here) add(url, 'link');
      } else if (el.tagName === 'IMG') {
        if (el.width >= MIN_IMG_PX && el.height >= MIN_IMG_PX) add(imgUrl(el), 'image');
      } else if (el.tagName === 'VIDEO') {
        add(videoUrl(el), 'video');
      }
    }
    return items;
  }

  async function copy(text) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.cssText = 'position:fixed;opacity:0';
      (document.body || document.documentElement).append(ta);
      ta.select();
      document.execCommand('copy');
      ta.remove();
    }
  }

  async function deliver(items) {
    if (!items.length) return say('주울 링크가 없어요');
    await copy(items.map(i => i.url).join('\n'));
    say(items.length === 1 ? '주웠어요!' : `${items.length}개 주웠어요!`);
    try {
      await chrome.runtime.sendMessage({
        t: 'capture',
        items,
        screen: {
          left: screen.availLeft, top: screen.availTop,
          width: screen.availWidth, height: screen.availHeight
        }
      });
    } catch (e) {
      say('확장이 새로 설치됐어요. 페이지를 새로고침해 주세요');
    }
  }

  function bubble() {
    if (hint) return hint;
    const host = document.createElement('div');
    host.style.cssText = 'all:initial;position:fixed;z-index:2147483647;right:20px;bottom:20px;pointer-events:none';
    const box = document.createElement('div');
    box.style.cssText = 'font:600 14px/1.4 "Malgun Gothic",sans-serif;color:#fff;background:#6A5BFF;' +
      'padding:9px 15px;border-radius:999px;box-shadow:0 6px 18px rgba(68,54,190,.35)';
    host.attachShadow({ mode: 'closed' }).append(box);
    (document.body || document.documentElement).append(host);
    hint = { host, box, timer: 0 };
    return hint;
  }

  function say(text, stay) {
    const h = bubble();
    h.box.textContent = text;
    clearTimeout(h.timer);
    if (!stay) h.timer = setTimeout(hideBubble, 1400);
  }

  function hideBubble() {
    if (hint) { hint.host.remove(); hint = null; }
  }

  function reset() {
    start = null;
    if (dragging) { dragging = false; hideBubble(); }
  }

  addEventListener('mousedown', e => {
    if (!alive() || !e.shiftKey || e.button !== 0) return;
    if (isEditable(e.composedPath()[0])) return;
    start = { x: e.clientX, y: e.clientY };
    e.preventDefault(); // 글자 선택·그림 끌기가 시작되지 않게
  }, true);

  addEventListener('mousemove', e => {
    if (!start || dragging) return;
    if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < DRAG_PX) return;
    dragging = true;
    say(`놓으면 이 페이지 링크 ${collectAll().length}개를 주워요 · Esc 취소`, true);
  }, true);

  addEventListener('mouseup', e => {
    if (!start) return;
    const wasDrag = dragging;
    reset();
    if (e.button !== 0 || !alive() || !wasDrag) return;
    swallowClick = true;
    setTimeout(() => { swallowClick = false; }, 80);
    deliver(collectAll());
  }, true);

  addEventListener('click', e => {
    if (!alive()) return;
    const stop = () => {
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    if (swallowClick) { swallowClick = false; return stop(); }
    if (!e.shiftKey || e.button !== 0 || isEditable(e.composedPath()[0])) return;
    const item = pick(e.clientX, e.clientY);
    if (!item) return;
    stop();
    deliver([item]);
  }, true);

  addEventListener('keydown', e => {
    if (e.key === 'Escape' && dragging) { reset(); swallowClick = true; }
  }, true);
  addEventListener('blur', reset);
})();
