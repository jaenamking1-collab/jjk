# FunETF 에서 펀드 이름으로 클래스별 펀드코드와 최근 기준가를 찾는다 (probe.yml 의 fundq 입력).
# fundq 형식: 검색어1|검색어2 ... 각 검색어로 filter/search 를 친다.
import json, os, urllib.parse, urllib.request
H = {'User-Agent': 'Mozilla/5.0', 'X-Requested-With': 'XMLHttpRequest', 'Referer': 'https://www.funetf.co.kr/search'}
def get(u):
    return urllib.request.urlopen(urllib.request.Request(u, headers=H), timeout=60).read().decode('utf-8', 'replace')
for q in os.environ['FUNDQ'].split('|'):
    for extra in ({}, {'schMoreClass': 'Y'}):
        p = dict(schVal=q, page=0, size=50, **extra)
        body = get('https://www.funetf.co.kr/api/public/product/fund/filter/search?' + urllib.parse.urlencode(p))
        print('==', q, extra, len(body))
        try:
            d = json.loads(body)
        except Exception:
            print(body[:600]); continue
        rows = d if isinstance(d, list) else next((v for v in d.values() if isinstance(v, list)), [])
        if not rows:
            print(json.dumps(d, ensure_ascii=False)[:600])
        for r in rows[:40]:
            print({k: r.get(k) for k in ('fundCd', 'repFundCd', 'fundFnm', 'fundNm', 'gijunGa', 'gijunYmd') if k in r})
