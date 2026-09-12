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


SOURCES = ['kodex', 'tiger', 'ace', 'plus', 'rise', 'sol']


def main():
    # ⚠️ getEtfNoticesAll 은 **캐시만** 읽는다(비어 있으면 stale:true 로 빈 배열). 링크를 보려면
    # 운용사별 getEtfNotices 를 불러 실제로 긁어오게 해야 한다(2026-09-12에 이걸로 헛돌았다).
    for sid in SOURCES:
        try:
            _, _, body = get(EXEC + '?action=getEtfNotices&source=' + sid, timeout=180)
            d = json.loads(body.decode('utf-8', 'replace'))
        except Exception as e:
            print('\n=== %s === ⛔ 목록 자체를 못 받음: %s' % (sid, e))
            continue
        if d.get('error'):
            print('\n=== %s === ⛔ 백엔드 오류: %s' % (sid, d['error']))
            continue
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
