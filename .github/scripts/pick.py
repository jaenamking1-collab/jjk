# 큰 JSON 응답에서 볼 부분만 판다.
#
# 왜: getDistributionAll 은 115KB 라 probe 의 `head -c 20000` 으로 자르면 뒤쪽 운용사
# (plus·rise·sol)가 아예 안 보인다. 2026-09-15 에 PLUS 를 확인하려다 매번 잘려서 만들었다.
#
# 쓰기: python3 .github/scripts/pick.py sources.plus /tmp/r.txt
#       점으로 파고들고, 숫자는 배열 인덱스로 본다.
import json
import sys

path_expr, file_path = sys.argv[1], sys.argv[2]
try:
    data = json.load(open(file_path, encoding='utf-8'))
except Exception as e:
    print('JSON 파싱 실패:', e)
    print(open(file_path, encoding='utf-8', errors='replace').read()[:1500])
    sys.exit(0)

for key in path_expr.split('.'):
    if isinstance(data, list):
        data = data[int(key)] if key.lstrip('-').isdigit() else None
    elif isinstance(data, dict):
        data = data.get(key)
    else:
        data = None
    if data is None:
        print(f'(없음: {key})')
        sys.exit(0)

# items 가 있는 응답은 건수와 나머지 키를 먼저 보여주고 앞 3건만 찍는다 — 그게 늘 알고 싶은 것이다.
if isinstance(data, dict) and isinstance(data.get('items'), list):
    items = data['items']
    rest = {k: v for k, v in data.items() if k != 'items'}
    print(f'items {len(items)}건 · 그 밖의 키: {json.dumps(rest, ensure_ascii=False)[:1200]}')
    for it in items[:3]:
        print('  ', json.dumps(it, ensure_ascii=False)[:300])
else:
    print(json.dumps(data, ensure_ascii=False)[:4000])
