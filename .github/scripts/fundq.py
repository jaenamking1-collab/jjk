# FunETF 전체 상품 목록에서 이름으로 펀드코드를 찾는다 (probe.yml 의 fundq 입력).
import json, os, urllib.request
H = {'User-Agent': 'Mozilla/5.0', 'X-Requested-With': 'XMLHttpRequest', 'Referer': 'https://www.funetf.co.kr/'}
req = urllib.request.Request('https://www.funetf.co.kr/api/public/quickSearch/product', headers=H)
items = json.load(urllib.request.urlopen(req, timeout=60))
print('전체', len(items), '예시', json.dumps(items[0], ensure_ascii=False)[:300])
for grp in os.environ['FUNDQ'].split('|'):
    keys = [k.strip() for k in grp.split(',') if k.strip()]
    hits = [it for it in items if all(k in json.dumps(it, ensure_ascii=False) for k in keys)]
    print('==', keys, len(hits))
    for it in hits[:25]:
        print(json.dumps(it, ensure_ascii=False)[:260])
