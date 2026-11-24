# 집 탭에게 주소를 대신 열어 달라고 하고 결과를 받는다(tools/tab_fetch.py 와 짝, 2026-11-24).
# 원격(클라우드) 세션·GitHub 서버처럼 해외 인터넷이라 국내 사이트가 막힐 때 쓴다. 이 저장소에 푸시할 수 있으면 어디서든 된다.
#   python tools/tabget.py <주소> [-o 저장할파일] [--referer 주소] [--wait 초]
# 탭은 1분마다 요청을 보므로 보통 1~2분 걸린다. 결과는 공개 브랜치(tab-fetch)에 남으니 개인 자료 주소는 넣지 마라.
import argparse, os, random, shutil, string, subprocess, sys, tempfile, time

REPO_URL = 'https://github.com/jaenamking1-collab/jjk.git'
BR = 'tab-fetch'

ap = argparse.ArgumentParser()
ap.add_argument('url')
ap.add_argument('-o', '--out')
ap.add_argument('--referer')
ap.add_argument('--wait', type=int, default=300)
a = ap.parse_args()

d = tempfile.mkdtemp(prefix='tabget_')
g = lambda *x: subprocess.run(['git', '-C', d] + list(x), capture_output=True, text=True)
try:
    if subprocess.run(['git', 'clone', '-q', '--depth', '1', '-b', BR, REPO_URL, d], capture_output=True).returncode:
        sys.exit('tab-fetch 브랜치를 못 받음')
    rid = time.strftime('%m%d%H%M%S') + ''.join(random.choice(string.ascii_lowercase) for _ in range(3))
    p = os.path.join(d, 'req.txt')
    lines = [l for l in open(p, encoding='utf-8').read().splitlines() if l.strip() and not l.startswith('#')][-19:]
    head = [l for l in open(p, encoding='utf-8').read().splitlines() if l.startswith('#')]
    lines.append(' '.join([rid, a.url] + ([a.referer] if a.referer else [])))
    open(p, 'w', encoding='utf-8', newline='\n').write('\n'.join(head + lines) + '\n')
    # 새로 받은 복사본엔 작성자 이름이 없을 수 있다(이 PC 는 저장소에만 설정) — 지금 폴더 설정을 빌리고, 없으면 기본값.
    for k, dv in (('user.name', 'jjk-tab'), ('user.email', 'jjk-tab@users.noreply.github.com')):
        v = subprocess.run(['git', 'config', k], capture_output=True, text=True).stdout.strip() or dv
        g('config', k, v)
    g('add', 'req.txt')
    if g('commit', '-qm', '탭에 열기 요청: ' + rid).returncode:
        sys.exit('요청 커밋 실패')
    for _ in range(5):
        if g('push', '-q', 'origin', 'HEAD:' + BR).returncode == 0:
            break
        g('pull', '-q', '--rebase', 'origin', BR)
    else:
        sys.exit('요청을 못 올림')
    print('요청', rid, '— 탭 답을 기다림(보통 1~2분)', file=sys.stderr)
    end = time.time() + a.wait
    while time.time() < end:
        time.sleep(15)
        g('fetch', '-q', '--depth', '1', 'origin', BR)
        meta = g('show', 'origin/%s:out/%s.txt' % (BR, rid))
        if meta.returncode == 0:
            print(meta.stdout, file=sys.stderr, end='')
            body = subprocess.run(['git', '-C', d, 'show', 'origin/%s:out/%s' % (BR, rid)], capture_output=True).stdout
            if a.out:
                open(a.out, 'wb').write(body)
                print('저장:', a.out, len(body), 'bytes', file=sys.stderr)
            else:
                sys.stdout.buffer.write(body)
            sys.exit(0)
    sys.exit('시간 안에 답이 없음 — 탭이 꺼졌을 수 있다(tab_alive.txt 확인)')
finally:
    shutil.rmtree(d, ignore_errors=True)
