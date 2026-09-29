# FunETF 기준가 API 를 최소 인자로 부를 수 있는지 확인한다 (probe.yml 의 fundq 입력: 펀드코드|펀드코드).
import json, os, urllib.parse, urllib.request
for cd in os.environ['FUNDQ'].split('|'):
    H = {'User-Agent': 'Mozilla/5.0', 'X-Requested-With': 'XMLHttpRequest', 'Referer': 'https://www.funetf.co.kr/product/fund/view/' + cd}
    for p in (dict(fundCd=cd, schNavMode='T'), dict(fundCd=cd, schNavMode='T', gijunYmd='20260928')):
        u = 'https://www.funetf.co.kr/api/public/product/view/fundnav?' + urllib.parse.urlencode(p)
        try:
            body = urllib.request.urlopen(urllib.request.Request(u, headers=H), timeout=60).read().decode()
            d = json.loads(body)
            print(cd, list(p), 'OK', [(r['gijunYmd'], r['gijunGa']) for r in d[:6]])
        except Exception as e:
            print(cd, list(p), 'ERR', e)
