# TIGER 공지 목록 중계 — 집 갤럭시탭(tools/tab_loop.sh, 매일 8~22시 매시)이 주로 돌고, 집·학교 PC 작업 스케줄러(tiger_relay_setup.bat)는 예비.
# need_fetch 는 이제 안 쓴다(매번 미래에셋을 연다, 2026-11-24). 함수는 진단용으로 남겨 둔다.
#
# 왜: 미래에셋 사이트가 2026-10-27 저녁부터 구글 서버(Apps Script) 요청을 403 으로 막는다.
# 금액·날짜는 예탁결제원(SEIBRO)으로 받지만 공시일과 공지 제목·링크는 미래에셋에만 있다.
# 국내 PC 에선 열리므로 여기서 목록을 받아 tiger_notices.json 으로 저장소에 올리고,
# 서버(Code.gs _tigerRelay)가 그 파일을 raw 로 읽어 공지 목록·공시일로 쓴다.
# 서버가 TIGER 를 제대로 읽고 있으면 아무것도 안 하고 끝난다(차단이 풀리면 저절로 멈춘다).
import html as html_lib, json, os, re, subprocess, sys, urllib.request
from datetime import datetime, timezone, timedelta

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, 'tiger_notices.json')
API = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec'
LIST = 'https://investments.miraeasset.com/tigeretf/ko/customer/notice/list.ajax'
VIEW = 'https://investments.miraeasset.com/tigeretf/ko/customer/notice/view.do?detailsKey='
KST = timezone(timedelta(hours=9))
PLUS_DIR = os.path.join(REPO, 'plus_relay')
PLUS_RAW = 'https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/plus_relay/'
WHO = '탭' if 'com.termux' in os.environ.get('PREFIX', '') else 'PC'   # 커밋 이름에 어디서 올렸는지


def log(msg):
    with open(os.path.join(REPO, 'tools', 'tiger_relay.log'), 'a', encoding='utf-8') as f:
        f.write(datetime.now(KST).strftime('%Y-%m-%d %H:%M ') + msg + '\n')


def ymd(s):
    m = re.search(r'(\d{4})\.(\d{2})\.(\d{2})', str(s or ''))
    return m.group(1) + m.group(2) + m.group(3) if m else ''


def need_fetch():
    """미래에셋을 열어야 하나? — 다른 운용사 분배 공지가 TIGER 최신 공지보다 새로 나왔을 때만.
    날짜를 정해 두는 것보다 낫다: 공시는 운용사들이 거의 같은 날(27·28일) 내니, 남들이 냈는데
    TIGER 만 없으면 그때 확인하고, TIGER 가 따라잡으면 다음 회차까지 미래에셋을 열지 않는다."""
    d = json.load(urllib.request.urlopen(API + '?action=getEtfNoticesAll', timeout=120))['sources']
    t = d.get('tiger') or {}
    if t.get('items') and not t.get('stale') and not t.get('fallback') and not t.get('relay'):
        return False, '서버가 TIGER 를 직접 읽고 있음'
    tiger_last = max([ymd(n.get('date')) for n in t.get('items') or []] + [''])
    try:
        prev = json.load(open(OUT, encoding='utf-8'))
        if not prev.get('dist'):
            return True, '중계본에 분배 본문(dist) 없음 — 처음 한 번 받기'
        tiger_last = max([tiger_last] + [ymd(n.get('date')) for n in prev.get('items', [])])
    except Exception:
        pass
    others = max([ymd(n.get('date')) for k, v in d.items() if k != 'tiger' for n in (v.get('items') or []) if '분배' in (n.get('title') or '')] + [''])
    if others <= tiger_last:
        return False, '다른 운용사 새 공지 없음(다른 곳 %s, TIGER %s)' % (others, tiger_last)
    return True, '다른 운용사 공지 %s > TIGER %s' % (others, tiger_last)


def fetch_list():
    body = b'firstIndex=0&listCnt=20&pageIndex=1&detailsKey=&q='
    rq = urllib.request.Request(LIST, data=body, headers={'Content-Type': 'application/x-www-form-urlencoded', 'User-Agent': 'Mozilla/5.0'})
    html = urllib.request.urlopen(rq, timeout=30).read().decode('utf-8')
    items = []
    for b in html.split('<li')[1:]:
        t = re.search(r'class="txt"[^>]*>([\s\S]*?)</', b)
        if not t:
            continue
        title = re.sub(r'\s+', ' ', re.sub(r'<[^>]+>', '', t.group(1))).strip()
        if '분배' not in title:
            continue
        k = re.search(r"'(\d+)'", b)
        d = re.search(r'class="item-date"[^>]*>([\s\S]*?)</', b)
        items.append({'title': title, 'date': re.sub(r'<[^>]+>', '', d.group(1)).strip() if d else '',
                      'url': VIEW + k.group(1) if k else '#'})
    return items


def _txt(s):
    return re.sub(r'\s+', ' ', re.sub(r'<[^>]+>', ' ', s).replace('&nbsp;', ' ')).strip()


def parse_view(html):
    """분배금 공지 본문 → 종목 표 + 일정. 서버 fetchDist_tiger_detail·parseTigerSchedule 와 같은 규칙.
    본문이 90KB 넘어 통째로는 못 올리니(서버 캐시 한 칸 100KB) 여기서 읽어 작게 올린다(2026-11-06)."""
    items = []
    for tr in re.findall(r'<tr[^>]*>([\s\S]*?)</tr>', html):
        cols = [_txt(c).replace('&#37;', '%').replace(',', '') for c in re.findall(r'<td[^>]*>([\s\S]*?)</td>', tr)]
        idx = next((i for i, c in enumerate(cols) if re.fullmatch(r'[0-9A-Z]{6}', c)), -1)
        if idx < 0:
            continue
        amount = rate = None
        for c in cols[idx + 2:]:
            if '%' in c:
                try:
                    rate = float(c.replace('%', '')) if rate is None else rate
                except ValueError:
                    pass
            elif re.fullmatch(r'\d[\d.]*\s*원?', c) and amount is None:
                amount = float(re.sub(r'[^\d.]', '', c))
        if amount is None:
            continue
        # 표 글자가 `S&#38;P500`(이중 이스케이프)·`TIGER코리아…`(띄어쓰기 없음)로 와서 화면에 그대로 나갔다(2026-11-11)
        nm = html_lib.unescape(html_lib.unescape(cols[idx + 1] if idx + 1 < len(cols) else ''))
        nm = re.sub(r'^TIGER(?=\S)', 'TIGER ', nm)
        items.append({'ticker': cols[idx], 'name': re.sub(r'\s*ETF\s*$', '', nm).strip(),
                      'amount': int(amount) if amount == int(amount) else amount, 'rate': rate})
    text = _txt(html)
    i = text.find('분배금 지급 일정')
    if i >= 0:
        text = text[i:]
    sched = {}
    for k, pat in (('공시일', r'분배금\s*공시일'), ('분배락일', r'분배락일'), ('기준일', r'분배금\s*지급기준일'), ('지급일', r'분배금\s*지급일')):
        m = re.search(r'(\d{1,2}[/.]\d{1,2})\s*\([월화수목금토일]\)\s*' + pat, text)
        if m:
            sched[k] = m.group(1).replace('.', '/')
    return items, sched


def body_text(html, title):
    """공지 본문을 표 칸은 ' | ' 로 이어 글자로. 제목부터 '목록보기' 앞까지(메뉴·바닥글 빼고)."""
    h = re.sub(r'(?is)<(script|style)[^>]*>.*?</\1>', ' ', html)
    h = re.sub(r'(?i)</t[dh]>', ' | ', h)
    h = re.sub(r'(?i)</tr>|<br\s*/?>|</p>|</li>', '\n', h)
    h = re.sub(r'<[^>]+>', ' ', h).replace('&nbsp;', ' ').replace('&amp;', '&').replace('&#37;', '%')
    h = re.sub(r'[ \t]+', ' ', h)
    h = re.sub(r' \|\s*\n', ' | ', re.sub(r'\n\s+', '\n', h))
    a = h.find(title[:20])
    b = h.find('목록보기', max(a, 0))
    return h[max(a, 0):b if b > 0 else None].strip()


def fetch_dist(notices):
    """목록 위쪽 분배금 공지 본문을 열어 회차(기준일의 '일' 20 이하=월중)별로 담는다. 최신순."""
    out = []
    for n in [x for x in notices if '분배금' in x['title']][:6]:
        key = (re.search(r'detailsKey=(\d+)', n['url']) or [None, None])[1]
        if not key:
            continue
        html = urllib.request.urlopen(urllib.request.Request(VIEW + key, headers={'User-Agent': 'Mozilla/5.0'}), timeout=30).read().decode('utf-8')
        items, sched = parse_view(html)
        if not items or not sched.get('기준일'):
            # 표기가 바뀌어 못 읽으면 본문 글자를 올리고 서버의 엑사원이 대신 읽는다(2026-11-09, 대조 88/88).
            txt = body_text(html, n['title'])
            if txt:
                out.append({'key': key, 'title': n['title'], 'date': n['date'], 'needAI': True, 'text': txt[:20000]})
                log('본문 표 못 읽음 → 엑사원에 맡김: ' + n['title'])
            continue
        cyc = '월중' if int(sched['기준일'].split('/')[1]) <= 20 else '월말'
        out.append({'key': key, 'title': n['title'], 'date': n['date'], 'cycle': cyc, 'sched': sched, 'items': items})
    return out


def relay_plus(force=False):
    """한화(PLUS) 사본 — plusetf.co.kr 이 구글 서버에서 주소조차 안 열릴 때(2026-11-09~) 목록과
    최신 월중·월말 분배금 공지(그림)를 plus_relay/ 에 올린다. 서버(Code.gs _plusMirrorGet)는 직접 읽기가
    실패할 때만 이걸 읽는다. 서버가 직접 읽고 있으면 한화 사이트를 열지 않는다."""
    if not force:
        p = json.load(urllib.request.urlopen(API + '?action=getEtfNoticesAll', timeout=120))['sources'].get('plus') or {}
        if p.get('items') and not p.get('stale') and not p.get('fallback') and not p.get('relay'):
            return
    get = lambda u: urllib.request.urlopen(urllib.request.Request(u, headers={'User-Agent': 'Mozilla/5.0'}), timeout=30).read()
    html = get('https://www.plusetf.co.kr/customer/notice/list').decode('utf-8', 'ignore')
    rows = []
    for n, body in re.findall(r'href="/customer/notice/detail\?n=(\d+)"[^>]*>([\s\S]*?)</a>', html)[:20]:
        cols = [re.sub(r'\s+', ' ', re.sub(r'<[^>]+>', '', c)).strip() for c in re.findall(r'<div[^>]*>([\s\S]*?)</div>', body)]
        rows.append((n, ' '.join(cols)))
    if not rows:
        log('PLUS 목록 0건 — 올리지 않음')
        return
    files = {'list.html': '\n'.join('<a href="/customer/notice/detail?n=%s"><div>%s</div></a>' % r for r in rows).encode('utf-8')}
    for cyc in ('월중', '월말'):   # 서버 파서와 같은 고르기: 회차별 최신 분배금 공지 1건
        hit = next((r for r in rows if 'PLUS ETF' in r[1] and '분배금' in r[1] and cyc in r[1]), None)
        if not hit:
            continue
        d = get('https://www.plusetf.co.kr/customer/notice/detail?n=' + hit[0]).decode('utf-8', 'ignore')
        parts = []
        for src in re.findall(r'''<img[^>]+src=["'](https?://[^"']*/upload/[^"']+\.(?:png|jpe?g))["']''', d, re.I):
            name = re.sub(r'[^A-Za-z0-9._-]', '', src.rsplit('/', 1)[1])
            files['upload/' + name] = get(src)
            parts.append('<img src="%supload/%s">' % (PLUS_RAW, name))
        parts += re.findall(r'<table[\s\S]*?</table>', d)   # 글자 표로 올리는 달 대비
        files['detail_%s.html' % hit[0]] = '\n'.join(parts).encode('utf-8')
    changed = False
    os.makedirs(os.path.join(PLUS_DIR, 'upload'), exist_ok=True)
    for rel, data in files.items():
        p = os.path.join(PLUS_DIR, rel)
        if not os.path.exists(p) or open(p, 'rb').read() != data:
            open(p, 'wb').write(data)
            changed = True
    for root, _, names in os.walk(PLUS_DIR):   # 지난 회차 사본은 지운다
        for nm in names:
            if os.path.relpath(os.path.join(root, nm), PLUS_DIR).replace(os.sep, '/') not in files:
                os.remove(os.path.join(root, nm))
                changed = True
    if not changed:
        return
    g = lambda *a: subprocess.run(['git', '-C', REPO] + list(a), capture_output=True, text=True)
    g('pull', '-q', '--rebase', '--autostash')
    g('add', '-A', 'plus_relay')
    r = g('commit', '-qm', 'PLUS 공지 사본(%s) — 최신 ' % WHO + rows[0][1][:50])
    p = g('push', '-q')
    log('PLUS 사본 올림 %d개, 최신 %s · commit %s · push %s' % (len(files), rows[0][1][:40], r.returncode, p.returncode or 'ok'))


RISE_DIR = os.path.join(REPO, 'rise_relay')


def relay_rise():
    """KB(kbam.co.kr) RISE 분배금 게시판 목록과 회차별 최신 공지 본문을 rise_relay/ 에 올린다.
    KB 는 2026-10-05부터 해외(구글) 서버를 막아 서버가 블로그로 버텼다 — 블로그는 KB 보다 20~55분 늦다(2026-11-24 실측).
    서버(Code.gs _riseGet)는 KB 가 안 열리면 이 사본을 읽고, saved.txt 가 20시간 넘게 묵으면 블로그로 돌아간다."""
    H = {'User-Agent': 'Mozilla/5.0'}
    get = lambda u, h=H: urllib.request.urlopen(urllib.request.Request(u, headers=h), timeout=30).read()
    lst = get('https://kbam.co.kr/support/notice/category/1', dict(H, RSC='1'))
    rows = re.findall(r'"id":(\d+),"category":\{"id":\d+,"name":"[^"]*"\},"title":"([^"]*)"[^{]*?"created_at":"(\d{4}-\d{2}-\d{2})', lst.decode('utf-8', 'ignore'))
    if not rows:
        log('RISE 목록 0건 — 올리지 않음')
        return
    rows.sort(key=lambda r: (r[2], int(r[0])), reverse=True)
    files = {'list.txt': lst, 'saved.txt': datetime.now(KST).strftime('%Y-%m-%d %H:%M').encode()}
    for cyc in ('월말', '월중'):   # 서버 _fetchDistRiseSite 와 같은 고르기: '말'=월말, 아니면 '중순|중'=월중, 회차별 최신 1건
        hit = next((r for r in rows if '분배금' in r[1] and re.search(r'\d{1,2}월', r[1])
                    and ('월말' if '말' in r[1] else '월중' if re.search('중순|중', r[1]) else '') == cyc), None)
        if hit:
            h = get('https://kbam.co.kr/support/notice/' + hit[0])
            files['n_%s.html' % hit[0]] = h
            # 일정(분배락·기준일·지급일)은 본문 글이 아니라 그림에만 있다 — 그림도 올린다(KB 그림 주소도 구글 서버엔 막힘).
            img = re.search(r'tiny-content[\s\S]*?<img[^>]+src="(/media/[^"]+)"', h.decode('utf-8', 'ignore'))
            if img:
                files['img_%s.png' % hit[0]] = get('https://kbam.co.kr' + img.group(1))
    os.makedirs(RISE_DIR, exist_ok=True)
    for rel, data in files.items():
        open(os.path.join(RISE_DIR, rel), 'wb').write(data)
    for nm in os.listdir(RISE_DIR):   # 지난 회차 사본은 지운다
        if nm not in files:
            os.remove(os.path.join(RISE_DIR, nm))
    g = lambda *a: subprocess.run(['git', '-C', REPO] + list(a), capture_output=True, text=True)
    g('pull', '-q', '--rebase', '--autostash')
    g('add', '-A', 'rise_relay')
    r = g('commit', '-qm', 'RISE 공지 사본(%s) — 최신 ' % WHO + rows[0][2] + ' ' + rows[0][1][:40])
    p = g('push', '-q')
    log('RISE 사본 올림 %d개, 최신 %s · commit %s · push %s' % (len(files), rows[0][1][:40], r.returncode, p.returncode or 'ok'))


def relay_fundnav():
    """IRP 펀드 기준가(FunETF)를 받아 fund_nav.txt 로 올린다. FunETF 가 구글 서버(2026-09-30)에 이어
    GitHub 서버까지 403 으로 막았다(2026-11-24) — 국내 집 인터넷인 탭이 받는다. 올리면 fund_nav.yml 이
    그 파일로 서버(updateFundNav)에 반영한다. 기준가는 공개 정보(그 작업 로그에도 원래 찍혔다)."""
    r = subprocess.run([sys.executable, os.path.join('.github', 'scripts', 'fundnav.py')], cwd=REPO,
                       capture_output=True, text=True, timeout=300)
    line = r.stdout.strip()
    if r.returncode or not re.fullmatch(r'K5\w+:\d{8}:[\d.]+(,K5\w+:\d{8}:[\d.]+)*', line):
        log('펀드 기준가 받기 실패: ' + (r.stderr.strip()[-200:] or line[:100]))
        return
    p = os.path.join(REPO, 'fund_nav.txt')
    if os.path.exists(p) and open(p, encoding='utf-8').read().strip() == line:
        return
    open(p, 'w', encoding='utf-8', newline='\n').write(line + '\n')
    g = lambda *a: subprocess.run(['git', '-C', REPO] + list(a), capture_output=True, text=True)
    g('pull', '-q', '--rebase', '--autostash')
    g('add', 'fund_nav.txt')
    c = g('commit', '-qm', 'IRP 펀드 기준가(%s) — %s' % (WHO, line.split(':')[1]))
    ps = g('push', '-q')
    log('펀드 기준가 올림 %s · commit %s · push %s' % (line.split(':')[1], c.returncode, ps.returncode or 'ok'))


def tab_alive_min():
    """탭이 마지막으로 '살아있음'을 올린 뒤 몇 분 지났나(모르면 큰 수)."""
    try:
        t = urllib.request.urlopen('https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/tab_alive.txt', timeout=30).read().decode('utf-8')
        at = datetime.strptime(t[:16], '%Y-%m-%d %H:%M').replace(tzinfo=KST)
        return (datetime.now(KST) - at).total_seconds() / 60
    except Exception:
        return 1e9


def main():
    # PC 는 예비다(2026-11-24): 탭이 2시간 안에 살아있음을 올렸으면 PC 는 아무것도 안 한다.
    # 둘이 같은 파일을 동시에 올려 커밋이 엉키는 걸 막고, 탭이 멈추면 PC 가 저절로 이어받는다.
    if WHO == 'PC' and '--force' not in sys.argv:
        m = tab_alive_min()
        if m <= 120:
            return
        log('탭 %s분째 소식 없음 — PC 가 대신 중계' % (int(m) if m < 1e8 else '?'))
    try:
        relay_rise()
    except Exception as e:
        log('RISE 사본 실패: %s' % e)
    try:
        relay_plus('--force' in sys.argv)
    except Exception as e:
        log('PLUS 사본 실패: %s' % e)
    try:
        relay_fundnav()
    except Exception as e:
        log('펀드 기준가 실패: %s' % e)
    # 예전엔 need_fetch 로 '다른 운용사가 먼저 공지했을 때만' 미래에셋을 열었다(PC 가 자주 꺼지던 때의 절약).
    # 그래서 TIGER 가 제일 먼저 공지하면 한 시간 넘게 늦었다. 탭이 늘 켜져 있으니 매번 연다(2026-11-24).
    # 목록 1번 + 본문 몇 번이고, 바뀐 게 없으면 커밋하지 않는다.
    items = fetch_list()
    if not items:
        log('목록 0건 — 올리지 않음')
        return
    old = None
    try:
        o = json.load(open(OUT, encoding='utf-8'))
        old = (o.get('items'), o.get('dist'))
    except Exception:
        pass
    try:
        dist = fetch_dist(items)
    except Exception as e:
        log('본문 읽기 실패: %s — 목록만 올림' % e)
        dist = (old or (None, None))[1] or []
    data = {'savedAt': datetime.now(KST).strftime('%Y-%m-%d %H:%M'), 'items': items[:20], 'dist': dist}
    if old == (data['items'], data['dist']):
        log('바뀐 공지 없음 (%d건)' % len(items))
        return
    json.dump(data, open(OUT, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    g = lambda *a: subprocess.run(['git', '-C', REPO] + list(a), capture_output=True, text=True)
    g('pull', '-q', '--rebase', '--autostash')
    g('add', 'tiger_notices.json')
    r = g('commit', '-qm', 'TIGER 공지 중계(%s) — 최신 %s %s' % (WHO, items[0]['date'], items[0]['title'][:30]))
    p = g('push', '-q')
    log('올림 %d건, 최신 %s · commit %s · push %s' % (len(items), items[0]['date'], r.returncode, p.returncode or 'ok'))


def upgrade_tab_loop():
    """탭에서 옛 tab_loop.sh(한 번 읽고 계속 돌아 고친 내용이 안 먹는 판)가 부른 거면, 새 루프를 띄우고 옛 루프를 끈다.
    새 루프는 TAB_LOOP_SELFEXEC=1 을 넘기고 매시간 자기 파일을 다시 읽으니 이 일은 한 번만 일어난다(2026-11-24)."""
    if WHO != '탭' or os.environ.get('TAB_LOOP_SELFEXEC'):
        return
    import signal
    parent = os.getppid()
    try:
        cmd = open('/proc/%d/cmdline' % parent, 'rb').read()
    except Exception:
        return
    if b'tab_loop.sh' not in cmd:
        return
    subprocess.Popen(['sh', os.path.join(REPO, 'tools', 'tab_loop.sh')], cwd=REPO, start_new_session=True,
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.kill(parent, signal.SIGTERM)
    log('옛 탭 루프(%d) → 새 루프로 갈아탐' % parent)


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        log('실패: %s' % e)
        raise
    finally:
        try:
            upgrade_tab_loop()
        except Exception as e:
            log('탭 루프 갈아타기 실패: %s' % e)
