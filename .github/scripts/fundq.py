# FunETF 펀드 클래스·기준가 찾기 (probe.yml 의 fundq 입력).
#   코드(K5…/KR5…) → 그 펀드 페이지의 클래스 목록과 클래스별 최근 기준가를 찍는다.
#   그 밖의 글자 → 전체 대표펀드 이름 목록에서 그 글자가 든 이름을 찍는다.
#   여러 개는 | 로 나눈다.
import json, os, re, urllib.parse, urllib.request
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
    if re.fullmatch(r'K[R5][0-9A-Z]{10}', q):
        html = get('https://www.funetf.co.kr/product/fund/view/' + q)
        t = re.search(r'<title>([^<]*)', html)
        print('==', q, t.group(1).strip() if t else '')
        for cd, title in re.findall(r'class-item" href="/product/fund/view/(\w+)">\s*<p class="class-title">([^<]*)', html):
            try: print('  ', cd, ' '.join(title.split()), nav(cd))
            except Exception as e: print('  ', cd, ' '.join(title.split()), 'ERR', e)
    else:
        if names is None:
            names = json.loads(get('https://www.funetf.co.kr/api/public/quickSearch/product', 'https://www.funetf.co.kr/'))
        print('== 이름', q, [n['repFundNm'] for n in names if q in (n.get('repFundNm') or '')][:15])
