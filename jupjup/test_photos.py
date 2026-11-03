import os, time, tempfile, threading, functools, http.server, base64, io
from PIL import Image
from playwright.sync_api import sync_playwright

HERE = os.path.dirname(os.path.abspath(__file__))
EXT = r'C:\Users\azsxd\Documents\jjk\jupjup'
SITE = os.path.join(tempfile.gettempdir(), 'jupjup-test-site')
os.makedirs(SITE, exist_ok=True)
for i in range(16):
    Image.new('RGB', (200, 150), ((i * 53) % 256, (i * 97) % 256, (i * 31) % 256)).save(f'{SITE}/p{i}.png')
Image.new('RGB', (400, 300), (10, 120, 200)).save(f'{SITE}/card.png')
for k in range(4):
    Image.new('RGB', (300, 290), (40 * k, 200 - 30 * k, 90)).save(f'{SITE}/g{k}.png')
Image.new('RGB', (400, 300), (200, 60, 90)).save(f'{SITE}/thr.png')
buf = io.BytesIO(); Image.new('RGB', (120, 90), (240, 200, 20)).save(buf, 'PNG')
DATA = 'data:image/png;base64,' + base64.b64encode(buf.getvalue()).decode()
imgs = ''.join(f'<img src="p{i}.png" width="120" height="90" alt="사진{i}">' for i in range(16))
serp = '<div id=serp style="width:560px;margin-top:20px">'
for i, n in enumerate(['첫번째', '두번째', '세번째']):
    u = f'https://site{i}.example.org/page/very/long/path/name'
    serp += (f'<div style="margin:14px 0"><a href="{u}" style="font-size:14px">Site {i}<br>https://site{i}.example.org › page › very › long › path › name</a>' if i == 0 else '<div style="margin:14px 0">')
    serp += f'<a href="{u}"><h3 style="font-size:22px;font-weight:400;margin:4px 0">{n} 검색 결과 제목입니다</h3></a><span style="font-size:14px">설명 글이 여기에 들어갑니다 조금 길게</span><img src="p{i + 3}.png" width="80" height="80"></div>'
serp += '</div>'
open(f'{SITE}/index.html', 'w', encoding='utf8').write(
    f'<!doctype html><meta charset=utf-8><body style="margin:20px"><h1>시험</h1><div id=g style="width:560px">{imgs}</div>'
    f'<p><a href="https://example.com/a">링크 하나 제목입니다</a></p>{serp}'
    '<div style="margin-top:20px"><a id=card href="https://example.com/post/1"><img src="card.png" width="200" height="150" alt="카드사진"></a></div>'
    '<div style="position:relative;width:200px;height:150px;margin-top:20px"><div><img src="thr.png" width="200" height="150" style="pointer-events:none"></div>'
    '<a id=thr href="https://example.com/post/2/media" style="position:absolute;inset:0"></a></div>'
    '<div id=gimg style="display:flex;gap:8px;margin-top:20px;width:400px">'
    '<div data-lpage="https://example.com/src/1"><img src="p0.png" width="120" height="90"><div>원본 글 제목 하나입니다</div></div>'
    '<div data-lpage="https://example.com/src/2"><img src="card.png" width="120" height="90"><div>원본 글 제목 둘입니다</div></div></div>'
    '<div id=gi style="display:grid;grid-template-columns:200px 200px;gap:10px;margin-top:20px">'
    + ''.join(f'<div data-lpage="https://shop.example.com/item/{k}"><div><img src="g{k}.png" width="200" height="193"></div>'
              f'<a href="https://shop.example.com/item/{k}?from=g" style="font-size:16px;display:block">상품 이름 {k}번 제목입니다</a></div>' for k in range(4))
    + '</div>'
    f'<div style="margin-top:20px"><img id=gdata src="{DATA}" width="120" height="90" data-src=""></div></body>')

srv = http.server.ThreadingHTTPServer(('127.0.0.1', 8765), functools.partial(http.server.SimpleHTTPRequestHandler, directory=SITE))
http.server.SimpleHTTPRequestHandler.log_message = lambda *a: None
threading.Thread(target=srv.serve_forever, daemon=True).start()
ok = []
def check(name, cond, info=''):
    ok.append(cond); print(('PASS ' if cond else 'FAIL ') + name, info)

with sync_playwright() as p:
    ctx = p.chromium.launch_persistent_context(tempfile.mkdtemp(dir=r'C:\Users\azsxd\pw-browsers'), headless=True, channel='chromium',
        accept_downloads=True, args=[f'--disable-extensions-except={EXT}', f'--load-extension={EXT}'])
    page = ctx.new_page(); page.goto('http://127.0.0.1:8765/index.html'); page.wait_for_timeout(800)
    def board():
        for _ in range(30):
            for pg in ctx.pages:
                if 'board.html' in pg.url: return pg
            time.sleep(0.1)
        # 헤드리스에서는 팝업 창이 안 열려 같은 주소를 직접 연다
        pg = ctx.new_page(); pg.goto(ctx.service_workers[0].url.replace('background.js', 'board.html')); return pg
    state = lambda b: b.evaluate("chrome.storage.local.get('board').then(d => d.board)")
    pend = lambda b: b.evaluate("chrome.storage.local.get('pending').then(d => (d.pending || []).map(i => i.kind + ' ' + i.url.slice(0, 40)))")
    def shift_click(sel):
        page.bring_to_front(); page.keyboard.down('Shift'); page.click(sel); page.keyboard.up('Shift')
    def shift_drag(sel):
        page.bring_to_front(); page.locator(sel).scroll_into_view_if_needed(); r = page.locator(sel).bounding_box()
        page.keyboard.down('Shift'); page.mouse.move(r['x'] - 5, r['y'] - 5); page.mouse.down()
        page.mouse.move(r['x'] + 100, r['y'] + 50, steps=5); page.mouse.move(r['x'] + r['width'] + 5, r['y'] + r['height'] + 5, steps=5)
        page.mouse.up(); page.keyboard.up('Shift')
    P = '.space:nth-child(2) .photo'

    # 1. 사진 Shift+클릭 → 바로 들어가지 않고 대기, 칸을 누르면 사진 칸으로
    shift_click('img[alt="사진0"]')
    b = board(); b.wait_for_timeout(800)
    check('사진은 대기', b.locator('.photo').count() == 0 and pend(b) == ['image http://127.0.0.1:8765/p0.png'], pend(b))
    check('대기 안내에 사진', '사진 1장' in b.inner_text('#dockText'), b.inner_text('#dockText'))
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1200)
    s = state(b); vis = s['shown'][:s['layout']]
    check('1칸→2칸', s['layout'] == 2, s['layout'])
    check('사진 칸이 둘째 칸', [i for i in vis if s['spaces'][i].get('photos')] == [vis[1]], vis)
    check('격자에 1장', b.locator(P).count() == 1)

    # 2. Shift+드래그 16장 → 칸 누르면 12장 채우고 새 판
    shift_drag('#g'); b.wait_for_timeout(800)
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)
    s = state(b); vis = s['shown'][:s['layout']]
    pics = [len([x for x in sp['items'] if x['kind'] == 'image']) for sp in s['spaces']]
    check('16장(중복 1 뺌)', sum(pics) == 16 and max(pics) == 12, pics)
    check('첫 칸은 그대로', pics[vis[0]] == 0, (vis, pics))
    check('넘친 사진은 새 판으로 둘째 칸에', s['spaces'][vis[1]].get('photos') and pics[vis[1]] == 4, (vis, pics))
    tops = b.eval_on_selector_all(P, 'els => els.map(e => Math.round(e.getBoundingClientRect().top))')
    check('한 줄 4장', len(tops) == 4 and len(set(tops)) == 1, tops)

    # 3. 롤오버·확대·내려받기·끌기·삭제
    b.hover(f'{P} >> nth=0'); b.wait_for_timeout(700)
    check('롤오버 크게', b.locator('#preview.big').is_visible())
    b.evaluate("window.__mark = document.querySelector('.space:nth-child(2) .photo'); window.__renders = 0; new MutationObserver(() => window.__renders++).observe(document.getElementById('spaces'), {childList: true, subtree: true})"); b.bring_to_front(); b.mouse.move(0, 0); b.hover(f'{P} >> nth=0'); b.wait_for_timeout(500); op = b.eval_on_selector(f'{P} .photo-acts', 'e => getComputedStyle(e).opacity'); check('x·내려받기 보임', op == '1', (op, b.evaluate("[window.__mark === document.querySelector('.space:nth-child(2) .photo'), window.__renders, getComputedStyle(document.querySelector('.space:nth-child(2) .photo-acts')).transition]")))
    b.click(f'{P} >> nth=1'); b.wait_for_timeout(200)
    check('클릭 확대', b.locator('#zoom').is_visible())
    b.click('#zoom'); b.wait_for_timeout(200)
    check('다시 클릭 닫힘', b.locator('#zoom').is_hidden())
    b.hover(f'{P} >> nth=0'); b.wait_for_timeout(200)
    with b.expect_download() as d:
        b.click(f'{P} >> nth=0 >> .down')
    check('내려받기', d.value.suggested_filename.endswith('.png'), d.value.suggested_filename)
    before = b.eval_on_selector_all(P, 'els => els.map(e => e.dataset.id)')
    b.drag_and_drop(f'{P} >> nth=0', f'{P} >> nth=2'); b.wait_for_timeout(500)
    after = b.eval_on_selector_all(P, 'els => els.map(e => e.dataset.id)')
    check('끌어 자리 바뀜', after == [before[1], before[2], before[0], before[3]])
    s = state(b)
    check('저장도 바뀐 순서', [x['id'] for x in s['spaces'][s['shown'][1]]['items']] == after)
    b.hover(f'{P} >> nth=0'); b.click(f'{P} >> nth=0 >> .del'); b.wait_for_timeout(300)
    check('삭제', b.locator(P).count() == 3)

    # 4. 구글형: 같은 주소 긴 주소 줄이 있어도 첫 제목 포함
    shift_drag('#serp'); b.wait_for_timeout(800)
    got = b.evaluate("chrome.storage.local.get('pending').then(d => d.pending.map(i => i.title))")
    check('구글형 첫 결과 포함', got == ['첫번째 검색 결과 제목입니다', '두번째 검색 결과 제목입니다', '세번째 검색 결과 제목입니다'], got)
    b.click('#dockCancel'); b.wait_for_timeout(300)

    # 5. 링크로 감싼 사진: 링크+사진 대기 → 칸 누르면 링크는 그 칸, 사진은 사진 칸
    n0 = b.locator('.photo').count()
    shift_click('#card img'); b.wait_for_timeout(800)
    check('링크+사진 대기', pend(b) == ['link https://example.com/post/1', 'image http://127.0.0.1:8765/card.png'], pend(b))
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)
    check('링크는 누른 칸', b.locator('.space >> nth=0 >> .row').count() == 1)
    check('사진은 사진 칸', b.locator('.photo').count() == n0 + 1)
    th = b.evaluate("chrome.storage.local.get('board').then(d => d.board.spaces.flatMap(s => s.items).filter(i => i.url.endsWith('card.png')).map(i => (i.thumb || '').slice(0, 22)))")
    check('사진 사본 저장', th == ['data:image/jpeg;base64'], th)

    # 6. 스레드형: 사진은 클릭을 안 받고 덮인 /media 링크 옆에 있다
    shift_click('#thr'); b.wait_for_timeout(800)
    check('스레드형 링크+사진', [x.split(' ')[0] for x in pend(b)] == ['link', 'image'], pend(b))
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)

    # 7. 구글 이미지형: 데이터로 넣어 둔 사진
    shift_click('#gdata'); b.wait_for_timeout(800)
    check('데이터 사진은 데이터 그대로', pend(b) == ['image data:image/png;base64,iVBORw0KGgoAAAANSU'], pend(b))
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)
    ok_img = b.evaluate("[...document.querySelectorAll('.photo img')].every(i => i.complete && i.naturalWidth > 0)")
    check('사진 칸에 깨진 사진 없음', ok_img)
    # 8. 구글 이미지형 드래그: 사진마다 원본 글 링크도 함께, 칸 누르면 링크는 그 칸·사진은 사진 칸
    shift_drag('#gimg'); b.wait_for_timeout(800)
    check('드래그 사진+원본 링크', pend(b) == ['link https://example.com/src/1', 'image http://127.0.0.1:8765/p0.png', 'link https://example.com/src/2', 'image http://127.0.0.1:8765/card.png'], pend(b))
    rows0 = b.locator('.space >> nth=0 >> .row').count()
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)
    check('원본 링크 2개는 누른 칸', b.locator('.space >> nth=0 >> .row').count() == rows0 + 2)
    t = b.evaluate("chrome.storage.local.get('board').then(d => d.board.spaces.flatMap(s => s.items).filter(i => i.url.includes('/src/')).map(i => i.title))")
    check('원본 링크 제목', t == ['원본 글 제목 하나입니다', '원본 글 제목 둘입니다'], t)
    # 9. 구글 이미지 칸: 위 두 칸은 통째로, 아래 두 칸은 사진 가운데만 네모 안 → 4칸 모두 링크+사진
    page.bring_to_front(); page.locator('#gi').scroll_into_view_if_needed()
    imgs = page.eval_on_selector_all('#gi img', 'els => els.map(e => { const r = e.getBoundingClientRect(); return [r.left, r.top, r.width, r.height] })')
    x0, y0 = imgs[0][0] - 5, imgs[0][1] - 5
    x1, y1 = imgs[3][0] + imgs[3][2] + 5, imgs[3][1] + imgs[3][3] / 2 + 15
    page.keyboard.down('Shift'); page.mouse.move(x0, y0); page.mouse.down(); page.mouse.move((x0 + x1) / 2, (y0 + y1) / 2, steps=5)
    page.mouse.move(x1, y1, steps=5); page.mouse.up(); page.keyboard.up('Shift'); b.wait_for_timeout(800)
    want = sum([[f'link https://shop.example.com/item/{k}', f'image http://127.0.0.1:8765/g{k}.png'] for k in range(4)], [])
    check('구글 이미지 칸 4개 링크+사진', pend(b) == want, pend(b))
    b.click('.space >> nth=0 >> .list'); b.wait_for_timeout(1500)
    b.screenshot(path=os.path.join(HERE, 'shot_board.png'))
    ctx.close()
srv.shutdown()
print('결과', sum(ok), '/', len(ok))
