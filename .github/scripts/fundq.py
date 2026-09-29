# FunETF 펀드 클래스·기준가 찾기 (probe.yml 의 fundq 입력).
#   코드(K5…/KR5…) → 그 펀드 페이지의 클래스 목록과 클래스별 최근 기준가를 찍는다.
#   그 밖의 글자 → 전체 대표펀드 이름 목록에서 그 글자가 든 이름을 찍는다.
#   여러 개는 | 로 나눈다.
import json, os, re, time, urllib.parse, urllib.request
UA = {'User-Agent': 'Mozilla/5.0'}
def get(u, ref=None):
    h = dict(UA, **({'X-Requested-With': 'XMLHttpRequest', 'Referer': ref} if ref else {}))
    return urllib.request.urlopen(urllib.request.Request(u, headers=h), timeout=60).read().decode('utf-8', 'replace')
def nav(cd):
    ref = 'https://www.funetf.co.kr/product/fund/view/' + cd
    d = json.loads(get('https://www.funetf.co.kr/api/public/product/view/fundnav?fundCd=' + cd + '&schNavMode=T', ref))
    return [(r['gijunYmd'][4:], r['gijunGa']) for r in d[:5]]
names = None
for q in os.environ['FUNDQ'].split('|'):
    q = q.strip()
    if q.startswith('xl:'):                # 전체 펀드 엑셀(클래스별 코드 포함)에서 이름으로 찾는다
        global_xl = globals().get('_xl')
        if global_xl is None:
            import io, subprocess
            import sys, site; subprocess.run([sys.executable, '-m', 'pip', '-q', 'install', '--user', '--break-system-packages', 'openpyxl'], capture_output=True); sys.path.append(site.getusersitepackages())
            import openpyxl
            raw = urllib.request.urlopen(urllib.request.Request('https://www.funetf.co.kr/api/public/download/excel/fundFilter', headers=UA), timeout=120).read()
            print('엑셀', len(raw), raw[:8])
            try:
                wb = openpyxl.load_workbook(io.BytesIO(raw), read_only=True)
                global_xl = [[str(c) for c in r if c is not None] for ws in wb.worksheets for r in ws.iter_rows(values_only=True)]
            except Exception as e:
                print('엑셀 못 읽음', e, raw[:300]); global_xl = []
            globals()['_xl'] = global_xl
            print('행', len(global_xl), global_xl[:2])
        keys = q[3:].split(',')
        hits = [r for r in global_xl if all(any(k in c for c in r) for k in keys)]
        print('== xl', keys, len(hits))
        for r in hits[:30]: print('  ', ' | '.join(r)[:200])
        continue
    q, _, want = q.partition(':')          # '코드:글자' 면 그 글자가 든 클래스만 기준가를 본다(429 방지)
    if re.fullmatch(r'K[R5][0-9A-Z]{10}', q):
        try: html = get('https://www.funetf.co.kr/product/fund/view/' + q)
        except Exception as e: print('==', q, 'ERR', e); continue
        t = re.search(r'<title>([^<]*)', html)
        print('==', q, t.group(1).strip() if t else '')
        for cd, title in re.findall(r'class-item" href="/product/fund/view/(\w+)">\s*<p class="class-title">([^<]*)', html):
            title = ' '.join(title.split())
            if want and want not in title: print('  ', cd, title); continue
            time.sleep(1)
            try: print('  ', cd, title, nav(cd))
            except Exception as e: print('  ', cd, title, 'ERR', e)
        time.sleep(2)
    else:
        if names is None:
            names = json.loads(get('https://www.funetf.co.kr/api/public/quickSearch/product', 'https://www.funetf.co.kr/'))
        print('== 이름', q, [n['repFundNm'] for n in names if q in (n.get('repFundNm') or '')][:15])
