# FunETF 전체 펀드 목록을 받아 이름으로 클래스별 펀드코드·기준가를 찾는다 (probe.yml 의 fundq 입력).
# fundq 형식: 'a,b|c,d' — 쉼표는 모두 포함, | 는 여러 묶음.
import json, os, urllib.parse, urllib.request
H = {'User-Agent': 'Mozilla/5.0', 'X-Requested-With': 'XMLHttpRequest', 'Referer': 'https://www.funetf.co.kr/search'}
rows, page = [], 0
while page < 40:
    u = 'https://www.funetf.co.kr/api/public/product/fund/filter/search?' + urllib.parse.urlencode(dict(page=page, size=2000))
    d = json.loads(urllib.request.urlopen(urllib.request.Request(u, headers=H), timeout=120).read())
    rows += d.get('content', [])
    if d.get('last', True): break
    page += 1
print('전체', len(rows), '쪽', page + 1)
for grp in os.environ['FUNDQ'].split('|'):
    keys = [k.strip() for k in grp.split(',') if k.strip()]
    hits = [r for r in rows if all(k in (r.get('fundFnm') or '') for k in keys)]
    print('==', keys, len(hits))
    for r in hits[:30]:
        print(r.get('fundCd'), r.get('gijunGa'), r.get('fundFnm'))
