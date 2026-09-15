# 큰 JSON 응답에서 볼 부분만 판다.
#
# 왜: getDistributionAll 은 115KB 라 probe 의 `head -c 20000` 으로 자르면 뒤쪽 운용사
# (plus·rise·sol)가 아예 안 보인다. 2026-09-15 에 PLUS 를 확인하려다 매번 잘려서 만들었다.
#
# 쓰기: python3 .github/scripts/pick.py sources.plus /tmp/r.txt
#       점으로 파고들고, 숫자는 배열 인덱스로 본다.
import json
import re
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

# 날짜 표기가 운용사마다 다르다: KODEX·PLUS 는 '9월 11일', TIGER 는 '9/11'.
# 처음엔 'N월' 만 찾았다가 TIGER 가 '-' 로 나와 **멀쩡한 걸 이상하다고 볼 뻔했다**(2026-09-15).
_MONTH_RE = re.compile(r'(\d{1,2})\s*월|(\d{1,2})\s*/\s*\d{1,2}')


def _month_of(text):
    m = _MONTH_RE.search(str(text))
    return (m.group(1) or m.group(2)) if m else None


def _months(obj):
    """제목·schedule(없으면 첫 항목의 sched)에 적힌 '월'을 모아 준다.
    오래된 회차를 물고 있는지 한눈에 보려는 것."""
    found = []
    mt = _month_of(obj.get('title') or '')
    if mt:
        found.append(mt + '월(제목)')
    # TIGER 처럼 top-level schedule 이 없는 곳은 항목의 sched 를 본다.
    sch = obj.get('schedule')
    where = 'schedule'
    if not isinstance(sch, dict) or not sch:
        first = (obj.get('items') or [{}])[0]
        sch = first.get('sched') if isinstance(first, dict) else None
        where = 'items[0]'
    if isinstance(sch, dict):
        for k, v in sch.items():
            mm = _month_of(v)
            if mm:
                found.append(f'{mm}월({k}·{where})')
                break
    return ' '.join(found) or '-'


# sources 처럼 '운용사 → 응답' 묶음이면 한 줄씩 요약한다. 6개사를 한 번에 훑으려고 만들었다
# (2026-09-15: PLUS 가 6월 기사를 물고 있던 걸 찾고 나서, 나머지도 같은지 봐야 했다).
if (isinstance(data, dict) and data
        and all(isinstance(v, dict) and 'items' in v for v in data.values())):
    print(f'{"운용사":<8}{"건수":>5}  {"월 표기":<28} 그 밖에')
    for name, v in data.items():
        flags = {k: x for k, x in v.items()
                 if k not in ('items', 'title', 'schedule', 'articleUrl', 'label')}
        print(f'{name:<8}{len(v.get("items") or []):>5}  {_months(v):<28} '
              f'{json.dumps(flags, ensure_ascii=False)[:160]}')
        t = v.get('title')
        if t:
            print(f'{"":<8}      제목: {t[:90]}')
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
