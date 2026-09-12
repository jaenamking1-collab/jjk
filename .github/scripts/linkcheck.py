#!/usr/bin/env python3
"""getEtfNoticesAll 이 주는 공지 링크를 하나씩 눌러 보고 결과를 찍는다.

확인하는 것:
  · HTTP 상태 (404/403/리다이렉트)
  · 최종 URL (목록 페이지로 튕기면 '상세로 안 간다'는 뜻이다)
  · <title> (엉뚱한 사이트면 여기서 드러난다)
"""
import json
import re
import urllib.request

EXEC = ('https://script.google.com/macros/s/'
        'AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec')
UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'


def get(url, timeout=40):
    req = urllib.request.Request(url, headers={'User-Agent': UA})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.getcode(), r.geturl(), r.read()


def main():
    code, _, body = get(EXEC + '?action=getEtfNoticesAll', timeout=180)
    data = json.loads(body.decode('utf-8', 'replace'))
    sources = data.get('sources') or data
    print('운용사 %d곳' % len(sources))

    for sid in sorted(sources):
        d = sources[sid] or {}
        items = (d.get('items') or [])[:3]   # 최근 3건이면 충분하다
        print('\n=== %s (%d건 중 %d건 확인) ===' % (sid, len(d.get('items') or []), len(items)))
        for it in items:
            url = it.get('url') or ''
            title = (it.get('title') or '')[:50]
            if not url or not url.startswith('http'):
                print('  ❌ URL 없음/이상: %r  ← %s' % (url, title))
                continue
            try:
                c, final, page = get(url)
                m = re.search(rb'<title[^>]*>(.*?)</title>', page, re.S | re.I)
                ptitle = (m.group(1).decode('utf-8', 'replace').strip()[:60] if m else '(title 없음)')
                moved = ' ⚠ 리다이렉트됨' if final.rstrip('/') != url.rstrip('/') else ''
                flag = '  ⛔' if c != 200 else ''
                print('  %s%s HTTP %s  %s' % (flag, moved, c, title))
                print('      요청: %s' % url)
                if moved:
                    print('      최종: %s' % final)
                print('      페이지 제목: %s' % ptitle)
            except Exception as e:
                print('  ⛔ 실패 %s  ← %s' % (e, title))
                print('      요청: %s' % url)


if __name__ == '__main__':
    main()
