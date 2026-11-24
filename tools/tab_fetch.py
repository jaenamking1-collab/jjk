# 밖(원격 세션·GitHub 서버·학교)에서 막히는 국내 사이트(KB·미래에셋·FunETF 등)를 집 탭이 대신 열어 준다(2026-11-24).
# tab_loop.sh 가 1분마다 부른다. 요청·결과는 저장소의 'tab-fetch' 브랜치로 주고받는다(main 을 어지럽히지 않게).
#   요청: req.txt 에 '아이디 주소 [Referer]' 한 줄씩 — 보통 tools/tabget.py 가 써 준다.
#   결과: out/<아이디>(받은 내용 그대로) + out/<아이디>.txt(상태코드·종류·최종 주소·크기·시각).
# 공개 저장소라 결과도 공개다 — 로그인 필요한 페이지·개인 자료는 요청하지 마라(받을 수도 없다: 쿠키를 안 쓴다).
import os, re, subprocess, sys, time, urllib.request
from datetime import datetime, timezone, timedelta

D = os.path.expanduser('~/jjk-fetch')
BR = 'tab-fetch'
MAX = 15 * 1024 * 1024
KST = timezone(timedelta(hours=9))


def g(*a):
    return subprocess.run(['git', '-C', D] + list(a), capture_output=True, text=True)


def main():
    if not os.path.isdir(os.path.join(D, '.git')):
        src = subprocess.run(['git', '-C', os.path.expanduser('~/jjk'), 'remote', 'get-url', 'origin'], capture_output=True, text=True).stdout.strip()
        if subprocess.run(['git', 'clone', '-q', '--depth', '1', '-b', BR, src, D], capture_output=True).returncode:
            return
        for k, dv in (('user.name', 'jjk-tab'), ('user.email', 'jjk-tab@users.noreply.github.com')):
            v = subprocess.run(['git', '-C', os.path.expanduser('~/jjk'), 'config', k], capture_output=True, text=True).stdout.strip() or dv
            g('config', k, v)
    g('fetch', '-q', '--depth', '1', 'origin', BR)
    if g('rev-parse', 'HEAD').stdout != g('rev-parse', 'origin/' + BR).stdout:
        g('reset', '-q', '--hard', 'origin/' + BR)
    reqs = []
    for ln in open(os.path.join(D, 'req.txt'), encoding='utf-8').read().splitlines():
        p = ln.split()
        if len(p) >= 2 and re.fullmatch(r'[A-Za-z0-9_-]{1,40}', p[0]) and re.match(r'https?://', p[1]):
            reqs.append(p)
    os.makedirs(os.path.join(D, 'out'), exist_ok=True)
    todo = [p for p in reqs if not os.path.exists(os.path.join(D, 'out', p[0] + '.txt'))]
    keep = {p[0] for p in reqs}
    gone = [f for f in os.listdir(os.path.join(D, 'out')) if f.split('.')[0] not in keep]
    if not todo and not gone:
        return
    for f in gone:
        os.remove(os.path.join(D, 'out', f))
    for p in todo:
        rid, url = p[0], p[1]
        h = {'User-Agent': 'Mozilla/5.0 (Linux; Android 11) AppleWebKit/537.36 Chrome/120 Safari/537.36'}
        if len(p) > 2:
            h['Referer'] = p[2]
        meta = {'url': url, 'at': datetime.now(KST).strftime('%Y-%m-%d %H:%M:%S')}
        body = b''
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=h), timeout=40) as r:
                body = r.read(MAX + 1)
                meta.update(status=r.status, type=r.headers.get('Content-Type', ''), final=r.geturl())
        except urllib.error.HTTPError as e:
            body = e.read(MAX + 1)
            meta.update(status=e.code, type=e.headers.get('Content-Type', ''), final=url)
        except Exception as e:
            meta.update(status='오류', error=str(e)[:300])
        if len(body) > MAX:
            body, meta['cut'] = body[:MAX], '15MB 넘어 잘림'
        meta['bytes'] = len(body)
        open(os.path.join(D, 'out', rid), 'wb').write(body)
        open(os.path.join(D, 'out', rid + '.txt'), 'w', encoding='utf-8').write('\n'.join('%s: %s' % kv for kv in meta.items()) + '\n')
    g('add', '-A')
    g('commit', '-qm', '탭 대신 열기: ' + ', '.join(p[0] for p in todo)[:80] if todo else '탭 대신 열기: 지난 결과 정리')
    for _ in range(3):
        if g('push', '-q', 'origin', 'HEAD:' + BR).returncode == 0:
            return
        g('pull', '-q', '--rebase', 'origin', BR)


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        print('tab_fetch 실패:', e, file=sys.stderr)
