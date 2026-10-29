# TIGER 공지 목록 중계 — 집·학교 PC 의 작업 스케줄러가 하루 2회 돈다(tiger_relay_setup.bat).
#
# 왜: 미래에셋 사이트가 2026-10-27 저녁부터 구글 서버(Apps Script) 요청을 403 으로 막는다.
# 금액·날짜는 예탁결제원(SEIBRO)으로 받지만 공시일과 공지 제목·링크는 미래에셋에만 있다.
# 국내 PC 에선 열리므로 여기서 목록을 받아 tiger_notices.json 으로 저장소에 올리고,
# 서버(Code.gs _tigerRelay)가 그 파일을 raw 로 읽어 공지 목록·공시일로 쓴다.
# 서버가 TIGER 를 제대로 읽고 있으면 아무것도 안 하고 끝난다(차단이 풀리면 저절로 멈춘다).
import json, os, re, subprocess, sys, urllib.request
from datetime import datetime, timezone, timedelta

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, 'tiger_notices.json')
API = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec'
LIST = 'https://investments.miraeasset.com/tigeretf/ko/customer/notice/list.ajax'
VIEW = 'https://investments.miraeasset.com/tigeretf/ko/customer/notice/view.do?detailsKey='
KST = timezone(timedelta(hours=9))


def log(msg):
    with open(os.path.join(REPO, 'tools', 'tiger_relay.log'), 'a', encoding='utf-8') as f:
        f.write(datetime.now(KST).strftime('%Y-%m-%d %H:%M ') + msg + '\n')


def server_ok():
    try:
        d = json.load(urllib.request.urlopen(API + '?action=getEtfNotices&source=tiger', timeout=120))
        return bool(d.get('items')) and not d.get('fallback') and not d.get('relay')
    except Exception as e:
        log('서버 확인 실패(그래도 받는다): %s' % e)
        return False


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


def main():
    if '--force' not in sys.argv and server_ok():
        log('서버가 TIGER 를 직접 읽고 있음 — 할 일 없음')
        return
    items = fetch_list()
    if not items:
        log('목록 0건 — 올리지 않음')
        return
    old = None
    try:
        old = json.load(open(OUT, encoding='utf-8')).get('items')
    except Exception:
        pass
    data = {'savedAt': datetime.now(KST).strftime('%Y-%m-%d %H:%M'), 'items': items[:20]}
    json.dump(data, open(OUT, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    if old == data['items']:
        log('바뀐 공지 없음 (%d건)' % len(items))
        return
    g = lambda *a: subprocess.run(['git', '-C', REPO] + list(a), capture_output=True, text=True)
    g('pull', '-q', '--rebase', '--autostash')
    g('add', 'tiger_notices.json')
    r = g('commit', '-qm', 'TIGER 공지 중계(PC) — 최신 %s %s' % (items[0]['date'], items[0]['title'][:30]))
    p = g('push', '-q')
    log('올림 %d건, 최신 %s · commit %s · push %s' % (len(items), items[0]['date'], r.returncode, p.returncode or 'ok'))


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        log('실패: %s' % e)
        raise
