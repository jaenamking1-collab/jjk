# Code.gs 의 FUND_NAV 에 적힌 펀드코드들의 최신 기준가를 FunETF 에서 받아
# 'code:yyyymmdd:기준가,…' 한 줄로 찍는다(fund_nav.yml 이 updateFundNav 에 넘긴다).
# 기준가는 공개 정보라 로그에 남아도 된다. 좌수·평가액은 여기서 다루지 않는다.
import json, re, sys, time, urllib.request

src = open('Code.gs', encoding='utf-8').read()
block = re.search(r'const FUND_NAV = \{(.*?)\};', src, re.S).group(1)
codes = list(dict.fromkeys(re.findall(r"'(K5\w+)'", block)))
out = []
for c in codes:
    url = f'https://www.funetf.co.kr/api/public/product/view/fundnav?fundCd={c}&schNavMode=T'
    for attempt in range(3):
        try:
            req = urllib.request.Request(url, headers={
                'User-Agent': 'Mozilla/5.0', 'X-Requested-With': 'XMLHttpRequest',
                'Referer': f'https://www.funetf.co.kr/product/fund/view/{c}'})
            rows = json.load(urllib.request.urlopen(req, timeout=30))
            r = rows[0]
            out.append(f"{c}:{r['gijunYmd']}:{r['gijunGa']}")
            break
        except Exception as e:
            print(c, '실패', e, file=sys.stderr)
            time.sleep(5 * (attempt + 1))
    else:
        sys.exit(f'{c} 기준가를 못 받았다')
    time.sleep(2)
print(','.join(out))
