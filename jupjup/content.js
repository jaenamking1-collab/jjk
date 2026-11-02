// 페이지 안: Shift+클릭 = 한 개, Shift+드래그 = 네모 안의 메인 제목들
(() => {
  if (window.__jupjupAlive && window.__jupjupAlive()) return;
  const alive = () => !!(chrome.runtime && chrome.runtime.id);
  window.__jupjupAlive = alive;

  const DRAG_PX = 8;
  const MIN_TITLE_CHARS = 6; // 평균 글자 수가 이보다 적은 묶음(메뉴·언론사 이름 등)은 제목이 아니다
  const NOT_TITLE = /^(\d{1,2}월\s*\d{1,2}일|\d{4}[.-]|\d{1,2}:\d{2}|\d+(분|시간|일|주|개월|년)\s*전)/; // 날짜·시각
  const TITLE_LABEL = 'h1,h2,h3,h4,h5,h6,[class*="title" i],[class*="headline" i]';
  let start = null, dragging = false, swallowClick = false, hint = null;
  let links = [], overlay = null, picked = [];

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

  const clean = t => (t || '').trim().replace(/\s+/g, ' ');

  // 카드처럼 여러 줄이 든 링크에서는 가장 긴 줄이 제목이다
  function longestLine(el) {
    return (el.innerText || '').split('\n').map(clean).sort((a, b) => b.length - a.length)[0] || '';
  }

  // 유튜브는 재생 위치 등이 달라도 v= 가 같으면 같은 영상이다
  function targetKey(href) {
    try {
      const url = new URL(href);
      return url.searchParams.get('v') || url.href;
    } catch (e) {
      return href;
    }
  }

  function linkTitle(a) {
    const label = a.querySelector(TITLE_LABEL);
    const heading = clean(label && label.innerText);
    if (heading) return heading;
    // 섬네일 링크에는 재생 시간 같은 글자만 있어, 같은 곳을 가리키는 링크 중 가장 긴 글자를 쓴다
    const key = targetKey(a.href);
    const same = [...document.querySelectorAll('a[href]')].filter(o => targetKey(o.href) === key);
    const texts = same.flatMap(o => [longestLine(o), clean(o.title)]).sort((x, y) => y.length - x.length);
    if (texts[0]) return texts[0];
    const img = a.querySelector('img');
    return clean(a.getAttribute('aria-label')) || clean(img && img.alt);
  }

  // 지메일 목록의 메일 줄은 링크가 아니다
  function gmailRow(el) {
    if (location.hostname !== 'mail.google.com') return null;
    const row = el.closest('tr.zA');
    const tag = row && row.querySelector('[data-legacy-thread-id]');
    if (!tag || el.closest('[role="checkbox"]')) return null;
    const subject = row.querySelector('.bog');
    return {
      url: `${location.origin}${location.pathname}#all/${tag.dataset.legacyThreadId}`,
      kind: 'link', title: clean(subject && subject.textContent), box: subject || row
    };
  }

  // 사진을 감싼 링크가 다른 곳으로 가면 게시글이다. 이 페이지나 사진·영상 파일을 가리키면 사진·영상이다
  function goesElsewhere(a) {
    const url = httpUrl(a.href);
    return !!url && url.split('#')[0] !== location.href.split('#')[0] &&
      !/\.(jpe?g|png|gif|webp|avif|bmp|mp4|webm)(\?|$)/i.test(url);
  }

  // SNS 는 사진이 링크로 감싸여 있지 않고 게시글 주소가 시간 표시에만 있어, 글 묶음을 올라가며 찾는다
  function snsPost(el) {
    const post = location.hostname.match(/(^|\.)(threads\.(com|net)|bsky\.app)$/) ? 'a[href*="/post/"]'
      : location.hostname.match(/(^|\.)instagram\.com$/) ? 'a[href*="/p/"],a[href*="/reel/"]'
      : location.hostname.match(/(^|\.)(x|twitter)\.com$/) ? 'a[href*="/status/"]' : '';
    if (!post) return null;
    let a = el.closest('a[href]');
    for (let e = el; !a && e && e !== document.body; e = e.parentElement) a = e.querySelector(post);
    return a && goesElsewhere(a) ? a : null;
  }

  function pick(x, y) {
    const stack = document.elementsFromPoint(x, y);
    const top = stack[0];
    if (!top) return null;
    const mail = gmailRow(top);
    if (mail) return { url: mail.url, kind: mail.kind, title: mail.title };
    const sns = snsPost(top);
    if (sns) {
      const url = httpUrl(sns.href);
      if (url) return { url, kind: 'link', title: '' };
    }
    const post = a => ({ url: httpUrl(a.href), kind: 'link', title: linkTitle(a) });
    const link = top.closest('a[href]');
    if (link && goesElsewhere(link)) return post(link);
    // 그림 위에 투명한 막이 덮인 사이트가 많아 아래쪽까지 본다
    for (const el of stack.slice(0, 6)) {
      const owner = el.closest('a[href]');
      if ((el.tagName === 'VIDEO' || el.tagName === 'IMG') && owner && goesElsewhere(owner)) return post(owner);
      if (el.tagName === 'VIDEO') return { url: videoUrl(el), kind: 'video', title: clean(document.title) };
      if (el.tagName === 'IMG') {
        const url = imgUrl(el);
        if (url) return { url, kind: 'image', title: clean(el.alt) || (owner ? linkTitle(owner) : '') };
      }
    }
    if (link) {
      const url = httpUrl(link.href);
      if (url) return { url, kind: 'link', title: linkTitle(link) };
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

  // 끌기 시작할 때 한 번만 재서 끄는 동안은 숫자만 비교한다
  function measureLinks() {
    const here = location.href.split('#')[0], out = [];
    for (const el of walk(document, [])) {
      if (el.tagName !== 'A' || !el.href) continue;
      const url = httpUrl(el.href);
      if (!url || url.split('#')[0] === here) continue;
      const head = el.querySelector(TITLE_LABEL);
      const label = head && head.innerText.trim() ? head : el;
      const text = label === el ? longestLine(el) : clean(label.innerText);
      const r = label.getBoundingClientRect();
      if (!text || !r.width || !r.height) continue;
      const style = getComputedStyle(label);
      out.push({
        url, text, left: r.left, top: r.top, width: r.width, height: r.height,
        size: parseFloat(style.fontSize), bold: +style.fontWeight >= 600
      });
    }
    for (const row of document.querySelectorAll('tr.zA')) {
      const mail = gmailRow(row);
      if (!mail) continue;
      const r = mail.box.getBoundingClientRect();
      if (r.width && r.height) {
        out.push({ url: mail.url, text: mail.title, left: r.left, top: r.top, width: r.width, height: r.height, size: 14, bold: false });
      }
    }
    return out;
  }

  // 글자 크기·굵기가 같은 링크끼리 묶어, 여러 개 반복되는 묶음 중 가장 큰 글자를 제목으로 본다
  function mainTitles(found) {
    const byUrl = new Map();
    for (const l of found) {
      if (!byUrl.has(l.url) || byUrl.get(l.url).text.length < l.text.length) byUrl.set(l.url, l);
    }
    const unique = [...byUrl.values()];
    const groups = new Map();
    for (const l of unique) {
      const key = l.size + (l.bold ? 'b' : '');
      if (!groups.has(key)) groups.set(key, { size: l.size, items: [], chars: 0 });
      const g = groups.get(key);
      g.items.push(l);
      g.chars += l.text.length;
    }
    const titled = [...groups.values()].filter(g => g.chars / g.items.length >= MIN_TITLE_CHARS);
    if (!titled.length) return [];
    const most = Math.max(...titled.map(g => g.items.length));
    const repeated = titled.filter(g => g.items.length >= most * 0.3);
    const size = Math.max(...repeated.map(g => g.size));
    return unique.filter(l => l.text.length >= MIN_TITLE_CHARS && !NOT_TITLE.test(l.text) &&
      titled.some(g => g.size >= size && g.items.includes(l)));
  }

  function linksInBox(box) {
    return links.filter(l => {
      const x = l.left + l.width / 2, y = l.top + l.height / 2;
      return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
    });
  }

  function showOverlay() {
    const host = document.createElement('div');
    host.style.cssText = 'all:initial;position:fixed;inset:0;z-index:2147483646;pointer-events:none';
    const root = host.attachShadow({ mode: 'closed' });
    const marks = document.createElement('div');
    const box = document.createElement('div');
    box.style.cssText = 'position:fixed;border:2px dashed #6A5BFF;border-radius:6px;background:rgba(106,91,255,.08)';
    root.append(marks, box);
    (document.body || document.documentElement).append(host);
    overlay = { host, marks, box };
  }

  function hideOverlay() {
    if (overlay) { overlay.host.remove(); overlay = null; }
  }

  function drawBox(x, y) {
    const box = {
      left: Math.min(start.x, x), right: Math.max(start.x, x),
      top: Math.min(start.y, y), bottom: Math.max(start.y, y)
    };
    Object.assign(overlay.box.style, {
      left: box.left + 'px', top: box.top + 'px',
      width: box.right - box.left + 'px', height: box.bottom - box.top + 'px'
    });
    picked = mainTitles(linksInBox(box));
    overlay.marks.replaceChildren(...picked.map(l => {
      const mark = document.createElement('div');
      mark.style.cssText = 'position:fixed;background:rgba(255,122,92,.22);border-radius:4px;' +
        `left:${l.left}px;top:${l.top}px;width:${l.width}px;height:${l.height}px`;
      return mark;
    }));
    say(picked.length ? `제목 ${picked.length}개 · Esc 취소` : '네모 안에 제목이 없어요', true);
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

  async function deliver(items, drag) {
    if (!items.length) return say('주울 링크가 없어요');
    await copy(items.map(i => i.url).join('\n'));
    say(items.length === 1 ? '주웠어요!' : `${items.length}개 주웠어요!`);
    try {
      await chrome.runtime.sendMessage({
        t: 'capture',
        items,
        drag,
        frameTitle: window === top ? '' : clean(document.title),
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
    if (dragging) { dragging = false; hideBubble(); hideOverlay(); }
    links = [];
  }

  addEventListener('mousedown', e => {
    if (!alive() || !e.shiftKey || e.button !== 0) return;
    if (isEditable(e.composedPath()[0])) return;
    start = { x: e.clientX, y: e.clientY };
    e.preventDefault(); // 글자 선택·그림 끌기가 시작되지 않게
  }, true);

  addEventListener('mousemove', e => {
    if (!start) return;
    if (!dragging) {
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < DRAG_PX) return;
      dragging = true;
      links = measureLinks();
      showOverlay();
    }
    drawBox(e.clientX, e.clientY);
  }, true);

  addEventListener('mouseup', e => {
    if (!start) return;
    const wasDrag = dragging;
    const items = picked.map(l => ({ url: l.url, kind: 'link', title: l.text }));
    reset();
    if (e.button !== 0 || !alive() || !wasDrag) return;
    swallowClick = true;
    setTimeout(() => { swallowClick = false; }, 80);
    deliver(items, true);
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
    deliver([item], false);
  }, true);

  addEventListener('keydown', e => {
    if (e.key === 'Escape' && dragging) { reset(); swallowClick = true; }
  }, true);
  addEventListener('blur', reset);
})();
