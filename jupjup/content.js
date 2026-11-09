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
  let links = [], images = [], overlay = null, picked = [];

  const httpUrl = u => {
    if (!u) return ''; // 빈 주소는 이 페이지 주소로 풀려 버린다
    try {
      const url = new URL(u, location.href);
      return /^https?:$/.test(url.protocol) ? url.href : '';
    } catch (e) { return ''; }
  };

  const isEditable = el => el instanceof Element &&
    (el.isContentEditable || el.closest('input,textarea,select,[contenteditable=""],[contenteditable="true"]'));

  function imgUrl(img) {
    const src = img.currentSrc || img.src;
    if (/^data:image\//.test(src)) return src; // 구글 이미지 검색은 사진을 데이터로 넣어 둔다
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
  // 화면에 안 보이는 글자(네이버의 '새 창 열림' 같은 화면 낭독기용 글자)는 뺀다
  const shown = e => { const r = e.getBoundingClientRect(); return r.width > 1 && r.height > 1; };

  function visibleLines(el) {
    let text = el.innerText || '';
    for (const e of el.querySelectorAll('*')) {
      const hidden = e.textContent.trim();
      if (hidden && !shown(e)) text = text.replace(hidden, '');
    }
    return text.split('\n').map(clean).filter(Boolean);
  }

  function longestLine(el) {
    return visibleLines(el).sort((a, b) => b.length - a.length)[0] || '';
  }

  // 그 글자를 실제로 그리는 가장 안쪽 칸(링크 상자가 넓거나 글자 크기가 링크와 다를 때)
  function textBox(el, line) {
    let box = el;
    for (const e of el.querySelectorAll('*')) if (shown(e) && clean(e.innerText).startsWith(line)) box = e;
    return box;
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

  // 게시글 링크를 고를 때 그 자리가 사진이면 사진도 같이 줍는다(링크는 칸을 눌러 넣고 사진은 사진 칸으로 바로 간다)
  // 스레드는 사진이 클릭을 받지 않고(pointer-events) 위에 덮인 /media 링크의 옆에 있어, 위로 올라가며 그 자리를 덮는 사진을 찾는다
  function withPhoto(link, stack, x, y) {
    const covers = el => { const r = el.getBoundingClientRect(); return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom; };
    let img = stack.slice(0, 6).find(el => el.tagName === 'IMG');
    for (let e = stack[0], n = 0; !img && e && e !== document.body && n < 8; e = e.parentElement, n++) {
      img = [...e.querySelectorAll('img')].find(covers);
    }
    const url = img && imgUrl(img);
    return url ? [link, { url, kind: 'image', title: clean(img.alt), post: link.url }] : [link];
  }

  // 사진이 나온 게시글: 감싼 링크 → 구글 이미지의 data-lpage → SNS 글 묶음
  function photoPost(img) {
    const a = img.closest('a[href]');
    if (a && goesElsewhere(a)) return { url: httpUrl(a.href), kind: 'link', title: linkTitle(a) };
    const lp = img.closest('[data-lpage]');
    if (lp && httpUrl(lp.dataset.lpage)) return { url: httpUrl(lp.dataset.lpage), kind: 'link', title: longestLine(lp) };
    const sns = snsPost(img);
    const url = sns && httpUrl(sns.href).replace(/\/media\/?$/, '');
    if (url) return { url, kind: 'link', title: '' };
    let here = location.href;
    try { if (window !== top && top.location.host === location.host) here = top.location.href; } catch (e) { /* 다른 사이트 틀 */ } // 네이버 블로그는 글이 틀(iframe) 안에 있다
    if (!/^https?:/.test(here)) return null;
    const card = cardLink(img, here);
    if (card) return card;
    // 다른 글로 가는 링크가 없으면 지금 보는 이 페이지가 그 게시글이다(상품·블로그 본문 사진). 제목은 h1, 없으면 페이지에서 읽는다
    const h1 = document.querySelector('h1');
    return { url: here, kind: 'link', title: clean(h1 && h1.innerText).slice(0, 200) };
  }

  // 링크 없이 눌러야만 이동하는 작은 상품 칸(네이버 '방금 본 상품' 등): 칸 속 숨은 주소, 없으면 상품명으로
  // 이 페이지의 그 자리를 가리키는 링크(#:~:text=)를 칸마다 따로 만든다. 큰 본문 사진은 칸으로 보지 않는다
  const LETTERS = /[A-Za-z가-힣]/g;
  function cardLink(img, here) {
    const ir = img.getBoundingClientRect();
    if (ir.width > 320) return null;
    let card = null;
    for (let e = img.parentElement, n = 0; e && e !== document.body && n < 6; e = e.parentElement, n++) {
      if (e.getBoundingClientRect().width > ir.width * 2.2) break;
      if ([...e.querySelectorAll('img')].filter(i => i.getBoundingClientRect().width >= 60).length > 1) break;
      card = e;
    }
    const letters = l => (l.match(LETTERS) || []).length;
    const line = card && visibleLines(card).filter(l => l.length >= MIN_TITLE_CHARS && !NOT_TITLE.test(l))
      .sort((a, b) => letters(b) - letters(a))[0];
    if (!line) return null;
    for (const e of [card, ...card.querySelectorAll('*')]) {
      for (const at of e.attributes) {
        if (/^(src|srcset)$/i.test(at.name) || !/^https?:\/\//.test(at.value)) continue;
        const url = httpUrl(at.value);
        if (url && !/\.(jpe?g|png|gif|webp|avif|svg)(\?|$)/i.test(url)) return { url, kind: 'link', title: line };
      }
    }
    return { url: here.split('#')[0] + '#:~:text=' + encodeURIComponent(line).replace(/-/g, '%2D'), kind: 'link', title: line };
  }

  function pick(x, y) {
    const stack = document.elementsFromPoint(x, y);
    const top = stack[0];
    if (!top) return null;
    const mail = gmailRow(top);
    if (mail) return [{ url: mail.url, kind: mail.kind, title: mail.title }];
    const sns = snsPost(top);
    if (sns) {
      const url = httpUrl(sns.href).replace(/\/media\/?$/, ''); // 스레드 사진 보기 주소 → 게시글 주소
      if (url) return withPhoto({ url, kind: 'link', title: '' }, stack, x, y);
    }
    const post = a => withPhoto({ url: httpUrl(a.href), kind: 'link', title: linkTitle(a) }, stack, x, y);
    const link = top.closest('a[href]');
    if (link && goesElsewhere(link)) return post(link);
    // 그림 위에 투명한 막이 덮인 사이트가 많아 아래쪽까지 본다
    for (const el of stack.slice(0, 6)) {
      const owner = el.closest('a[href]');
      if ((el.tagName === 'VIDEO' || el.tagName === 'IMG') && owner && goesElsewhere(owner)) return post(owner);
      if (el.tagName === 'VIDEO') return { url: videoUrl(el), kind: 'video', title: clean(document.title) };
      if (el.tagName === 'IMG') {
        const url = imgUrl(el);
        const post = url && photoPost(el);
        if (url) return [post, { url, kind: 'image', title: clean(el.alt) || (owner ? linkTitle(owner) : ''), post: post ? post.url : '' }].filter(Boolean);
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

  // 구글 이미지 칸의 링크는 사진의 원본 주소(data-lpage)로 맞춰 제목과 사진이 한 링크가 되게 한다
  function linkUrl(a) {
    const lp = a.closest('[data-lpage]');
    return httpUrl(lp ? lp.dataset.lpage : a.href);
  }

  // 끌기 시작할 때 한 번만 재서 끄는 동안은 숫자만 비교한다
  function measureLinks() {
    const here = location.href.split('#')[0], out = [];
    for (const el of walk(document, [])) {
      if (el.tagName !== 'A' || !el.href) continue;
      const url = linkUrl(el);
      if (!url || url.split('#')[0] === here) continue;
      const head = el.querySelector(TITLE_LABEL);
      const text = head && head.innerText.trim() ? clean(head.innerText) : longestLine(el);
      const label = head && head.innerText.trim() ? head : text ? textBox(el, text) : el;
      const r = label.getBoundingClientRect();
      if (!text || !r.width || !r.height) continue;
      const style = getComputedStyle(label);
      out.push({
        url, text, el, left: r.left, top: r.top, width: r.width, height: r.height,
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
    // 같은 주소가 여러 번 있으면(구글 첫 결과의 사이트 주소 줄 등) 글이 긴 쪽이 아니라 제목다운 큰 글자를 남긴다
    const titleLike = l => l.text.length >= MIN_TITLE_CHARS && !NOT_TITLE.test(l.text);
    const better = (a, b) => titleLike(a) !== titleLike(b) ? titleLike(a)
      : a.size !== b.size ? a.size > b.size : a.text.length > b.text.length;
    const byUrl = new Map();
    for (const l of found) {
      if (!byUrl.has(l.url) || better(l, byUrl.get(l.url))) byUrl.set(l.url, l);
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

  // 드래그는 칸(제목+사진) 단위로 줍는다(사용자 지정 2026-11-03). 사진 가운데나 제목 가운데가 네모 안이면 그 칸을 줍고,
  // 칸마다 게시글 링크와 사진을 둘 다 줍는다. 이보다 작은 사진은 썸네일(검색·뉴스 목록)로 보고 링크만 줍는다
  const BIG_W = 150, BIG_H = 120;
  const isBig = r => r.width >= BIG_W && r.height >= BIG_H;

  // 제목 링크가 든 칸의 큰 사진. 다른 글의 제목이 섞이는 높이까지는 올라가지 않는다
  function cardImage(a) {
    const other = e => [...e.querySelectorAll('a[href]')].some(x => linkUrl(x) !== linkUrl(a) && longestLine(x).length >= MIN_TITLE_CHARS);
    for (let e = a, n = 0; e && e !== document.body && n < 6; e = e.parentElement, n++) {
      if (n && other(e)) return null;
      const big = [...e.querySelectorAll('img')].filter(i => isBig(i.getBoundingClientRect()));
      if (big.length) return big.length === 1 ? big[0] : null;
    }
    return null;
  }

  // 네모 안의 사진(작은 아이콘은 뺀다). 같은 사진이 여러 곳에 있을 수 있어 겹침은 놓을 때 거른다
  function measureImages() {
    const out = [];
    for (const el of walk(document, [])) {
      if (el.tagName !== 'IMG') continue;
      const url = imgUrl(el), r = el.getBoundingClientRect();
      if (!url || r.width < 60 || r.height < 60) continue;
      out.push({ url, text: clean(el.alt), kind: 'image', el, left: r.left, top: r.top, width: r.width, height: r.height });
    }
    return out;
  }

  function linksInBox(box, list = links) {
    return list.filter(l => {
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
    const titles = mainTitles(linksInBox(box));
    const inBox = linksInBox(box, images);
    const big = inBox.filter(isBig);
    // 제목도 큰 사진도 없으면(작은 사진만 모인 화면) 예전처럼 사진을 줍는다
    picked = titles.concat(big.length || titles.length ? big : inBox).sort((a, b) => Math.abs(a.top - b.top) > 4 ? a.top - b.top : a.left - b.left); // 소수점 차이는 같은 줄로 본다
    overlay.marks.replaceChildren(...picked.map(l => {
      const mark = document.createElement('div');
      mark.style.cssText = 'position:fixed;background:rgba(255,122,92,.22);border-radius:4px;' +
        `left:${l.left}px;top:${l.top}px;width:${l.width}px;height:${l.height}px`;
      return mark;
    }));
    const pics = picked.filter(l => l.kind === 'image').length, heads = picked.length - pics;
    const what = [heads && `제목 ${heads}개`, pics && `사진 ${pics}장`].filter(Boolean).join(' · ');
    say(picked.length ? `${what} · Esc 취소` : '네모 안에 제목·사진이 없어요', true);
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
    // 클릭은 앞의 것(게시글 링크)만 복사한다. 함께 주운 사진은 사진 칸으로 간다
    await copy(drag ? items.map(i => i.url).join('\n') : items[0].url);
    say(drag ? `${items.length}개 주웠어요!` : items.length > 1 ? '링크와 사진을 주웠어요!' : '주웠어요!');
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
      images = measureImages();
      showOverlay();
    }
    drawBox(e.clientX, e.clientY);
  }, true);

  addEventListener('mouseup', e => {
    if (!start) return;
    const wasDrag = dragging;
    // 칸마다 링크와 사진을 둘 다: 사진은 그 게시글 링크를, 제목은 그 칸의 큰 사진을 같이(겹치는 주소는 한 번)
    const seen = new Set();
    // 사진에는 함께 주운 링크 주소(post)를 붙여, 그 링크를 지우면 사진도 같이 지운다
    const photoOf = (img, post) => { const url = img && imgUrl(img); return url && { url, kind: 'image', title: clean(img.alt), post }; };
    const items = picked.flatMap(l => {
      if (l.kind !== 'image') return [{ url: l.url, kind: 'link', title: l.text }, l.el && photoOf(cardImage(l.el), l.url)];
      const post = photoPost(l.el);
      return [post, { url: l.url, kind: 'image', title: l.text, post: post ? post.url : '' }];
    })
      .filter(i => i && !seen.has(i.url) && seen.add(i.url));
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
    deliver([].concat(item), false);
  }, true);

  addEventListener('keydown', e => {
    if (e.key === 'Escape' && dragging) { reset(); swallowClick = true; }
  }, true);

  // 사진 우클릭 막힘 풀기: 페이지가 사진 우클릭을 막으면 '사진 저장' 메뉴도 안 뜬다.
  // 사진 위에서만 페이지가 우클릭을 못 받게 한다(문서·지도 같은 앱의 자체 메뉴는 그대로).
  for (const type of ['contextmenu', 'mousedown', 'mouseup']) {
    addEventListener(type, e => {
      if (!alive() || (type !== 'contextmenu' && e.button !== 2)) return;
      if (e.composedPath()[0] instanceof HTMLImageElement) e.stopImmediatePropagation();
    }, true);
  }
  addEventListener('blur', reset);
})();
