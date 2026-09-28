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

# 특수 모드: 'sched' — 6개사의 회차별 일정(공시·분배락·기준·지급)을 중복 없이 모아 준다.
# 왜: "예상 공시일·기준일을 알 수 있나"(2026-09-21)에 답하려면 과거 회차가 실제로 규칙적인지
# 봐야 하는데, 응답이 110KB 라 items 를 눈으로 훑을 수가 없다. 회차 단위로 접어서 본다.
if path_expr == 'sched':
    import json as _j
    d = _j.load(open(file_path, encoding='utf-8'))
    srcs = d.get('sources') or {'(단일)': d}
    for name, v in srcs.items():
        seen = {}
        for it in (v.get('items') or []):
            s = it.get('sched') or {}
            if not s:
                continue
            key = (it.get('cycle') or '?',
                   s.get('공시일', '-'), s.get('분배락일', '-'),
                   s.get('기준일', '-'), s.get('지급일', '-'))
            seen[key] = seen.get(key, 0) + 1
        print(f'--- {name} ({len(seen)}회차) ---')
        for (cyc, pub, ex, base, pay), n in sorted(seen.items(), key=lambda x: x[0][3]):
            print(f'  {cyc:<4} 공시 {pub:<9} 분배락 {ex:<9} 기준 {base:<9} 지급 {pay:<9} ({n}종목)')
    sys.exit(0)

# 특수 모드: 'chg' — 공지 종목 표의 '전달비'를 화면과 같은 규칙으로 다시 계산해 전 행을 찍는다.
# 왜: "전달비에 심각한 숫자가 있다"(2026-09-28)를 확인하려면 6개사 전 종목을 봐야 하는데 응답이 67KB 다.
# 규칙은 dist_notice.html renderDistGrid 의 prevAmt/chgHtml 과 같다(기준일 월로 이번 달/지난달을 가른다).
if path_expr == 'chg':
    d = json.load(open(file_path, encoding='utf-8'))
    _mre = re.compile(r'(\d{1,2})\s*월|(\d{1,2})\s*[/.]\s*\d{1,2}')
    _dre = re.compile(r'(\d{1,2})\s*월\s*(\d{1,2})\s*일|\d{1,2}\s*[/.]\s*(\d{1,2})')

    def mon(s):
        m = _mre.search(str(s or ''))
        return int(m.group(1) or m.group(2)) if m else None

    def cyc(it, rep):
        if it.get('cycle'):
            return it['cycle']
        s = it.get('sched') or rep or {}
        m = _dre.search(str(s.get('기준일') or s.get('지급일') or ''))
        if not m:
            return ''
        return '월중' if int(m.group(2) or m.group(3)) <= 20 else '월말'

    import datetime
    sel = int(sys.argv[3]) if len(sys.argv) > 3 else datetime.date.today().month
    prev = 12 if sel == 1 else sel - 1
    for name, v in (d.get('sources') or {}).items():
        rep = v.get('schedule') or {}
        its = v.get('items') or []
        sch = lambda it: it.get('sched') or rep
        pm, pr = {}, {}
        for it in its:
            if it.get('hist') and it.get('amount') is not None and mon(sch(it).get('기준일') or sch(it).get('지급일')) == prev:
                pm[(it.get('ticker') or it.get('name')) + '|' + cyc(it, rep)] = it['amount']
                pr[(it.get('ticker') or it.get('name')) + '|' + cyc(it, rep)] = it.get('rate')
        print(f'--- {name} ({sel}월, 지난달 비교 {len(pm)}건) ---')
        for it in its:
            if it.get('hist') or mon(sch(it).get('기준일') or sch(it).get('지급일')) != sel:
                continue
            c = cyc(it, rep)
            p = pm.get((it.get('ticker') or it.get('name')) + '|' + c)
            a = it.get('amount')
            pct = f'{(a - p) / p * 100:+.1f}%' if p and a is not None else '-'
            flag = ' ⚠️' if p and a is not None and abs((a - p) / p) > 0.3 else ''
            rt = f"  분배율 {it.get('rate')}% / 전달 {pr.get((it.get('ticker') or it.get('name')) + '|' + c)}%" if flag else ''
            print(f'  {c or "?":<3} {it.get("ticker", ""):<7} {str(it.get("name", ""))[:26]:<26} '
                  f'{a!s:>6} 전달 {p!s:>6} {pct:>8}{flag}  기준 {sch(it).get("기준일", "-")}{rt}')
    sys.exit(0)
# 특수 모드: 'raw' — 응답 전체를 한 줄로 찍는다. 원격 세션이 로그에서 받아 화면을 재현하는 데 쓴다.
if path_expr == 'raw':
    print('RAW ' + open(file_path, encoding='utf-8', errors='replace').read().replace('\n', ' '))
    sys.exit(0)

# 특수 모드: 'rss' — RSS 글 목록(날짜·글번호·제목)만 찍는다. text 모드는 CDATA 제목을 태그로 보고 지워 버린다.
# 왜: SOL 은 네이버 블로그 RSS 로 분배 공지를 찾는데, 9월말 글을 못 찾는 이유를 보려면 목록이 필요했다(2026-09-28).
if path_expr == 'rss':
    raw = open(file_path, encoding='utf-8', errors='replace').read()
    for c in raw.split('<item>')[1:15]:
        t = re.search(r'<title>\s*(?:<!\[CDATA\[)?([\s\S]*?)(?:\]\]>)?\s*</title>', c)
        d = re.search(r'<pubDate>\s*(.*?)\s*</pubDate>', c)
        l = re.search(r'/(\d{6,})', c)
        print((d.group(1) if d else '-'), (l.group(1) if l else '-'), (t.group(1).strip() if t else '-'))
    sys.exit(0)

# 특수 모드: 'text:<낱말>' — HTML 응답의 태그를 벗기고 그 낱말 앞뒤만 찍는다.
# 왜: 운용사 공지 원문(TIGER view.do 등)은 수십 KB 라 head 로 자르면 본문까지 닿지 않는다(2026-09-28).
if path_expr.startswith('text:'):
    word = path_expr[5:]
    span = 300
    if '@' in word:                      # text:낱말@3000 — 뒤로 더 길게 본다(표 전체가 필요할 때)
        word, span = word.rsplit('@', 1)
        span = int(span)
    raw = open(file_path, encoding='utf-8', errors='replace').read()
    txt = re.sub(r'\s+', ' ', re.sub(r'<[^>]+>', ' ', raw).replace('&nbsp;', ' '))
    hits = [m.start() for m in re.finditer(re.escape(word), txt)]
    print(f'"{word}" {len(hits)}곳')
    for h in hits[:8]:
        print('  …' + txt[max(0, h - 200):h + span] + '…')
    sys.exit(0)
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
