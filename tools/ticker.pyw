# 바탕화면 실시간 시세 위젯. 드래그로 이동, 톱니로 설정.
# 목록 한 줄에 하나: "심볼" 또는 "심볼=표시이름", "---" 는 구분선.
# 국내(6자리 코드, KOSPI/KOSDAQ)는 네이버 실시간, 해외/코인/환율은 야후.
import ctypes, ctypes.wintypes, datetime, json, os, re, shutil, subprocess, sys, time, tkinter as tk
import urllib.error, urllib.parse, urllib.request
from threading import Thread

RAW = "https://raw.githubusercontent.com/jaenamking1-collab/jjk/main/tools/ticker.pyw"
update_note = ["확인 안 함"]             # 설정창에 보여줄 마지막 갱신 결과


def repo_of(path):
    """이 파일이 들어 있는 git 저장소의 위치(없으면 None)."""
    d = os.path.dirname(os.path.abspath(path))
    while True:
        if os.path.isdir(os.path.join(d, ".git")):
            return d
        up = os.path.dirname(d)
        if up == d:
            return None
        d = up


def self_update():
    """켤 때마다 최신 코드로 맞춘다. 사람이 명령을 외워서 칠 필요가 없어야 한다.
    - 저장소 안(내 PC): `git pull --ff-only` — 작업 중인 변경이 있으면 pull이 그냥 실패하므로
      내가 고치던 코드가 덮일 일이 없다.
    - 저장소 밖(남에게 준 사본): GitHub raw에서 받아 자기 자신을 갈아끼운다.
    파일이 실제로 바뀐 경우에만 새 코드로 다시 시작한다."""
    me = os.path.abspath(__file__)
    try:
        before = open(me, "rb").read()
    except Exception:
        return
    repo = repo_of(me)
    if repo:
        ok = git(repo, "pull", "--ff-only", "--quiet") == 0
        update_note[0] = "최신" if ok else "확인 실패(작업 중이거나 오프라인)"
    else:
        try:
            new = urllib.request.urlopen(RAW, timeout=8).read()
            if len(new) > 10000 and b"tkinter" in new and new != before:
                # 문법이 깨진 코드는 받아도 쓰지 않는다. 파이썬은 파일 전체를 컴파일한 뒤
                # 실행하므로, 한 번 덮이면 이 갱신 코드조차 돌지 못해 영영 회복이 안 된다.
                compile(new, me, "exec")
                with open(me, "wb") as f:
                    f.write(new)
            update_note[0] = "최신"
        except SyntaxError:
            update_note[0] = "새 코드에 문제가 있어 그대로 둠"
        except Exception:
            update_note[0] = "확인 실패(인터넷)"    # 인터넷이 없으면 있던 걸로 그냥 돈다
    try:
        if open(me, "rb").read() != before:
            os.execv(sys.executable, [sys.executable, me])
    except Exception:
        pass


def git(repo, *args, timeout=15):
    """조용한 git 호출. 실패해도 위젯은 그냥 뜬다(설정 동기화는 부가 기능일 뿐)."""
    try:
        return subprocess.run(("git", "-C", repo) + args, timeout=timeout,
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                              creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0)).returncode
    except Exception:
        return 1


self_update()


def sync():
    """종목 목록을 다른 PC와 맞춘다. 켤 때 한 번, 목록을 고칠 때 한 번.
    Claude Code 훅에만 기대면 그 폴더에서 세션을 안 연 날은 설정이 그대로 어긋난다
    (2026-08-24: 집 PC가 사흘째 옛 목록을 보고 있었다). 위젯이 직접 챙긴다.
    양쪽이 목록을 고쳤으면 다른 PC 것을 따르고, 내 것은 .conflict 파일로 옆에 남긴다."""
    if not SHARED:
        return
    repo = os.path.dirname(os.path.dirname(SHARED))
    git(repo, "add", "ticker")
    git(repo, "commit", "-m", "ticker: 종목 목록 변경")     # 바뀐 게 없으면 조용히 실패
    if git(repo, "pull", "--rebase", "--quiet", "origin", "main"):
        git(repo, "rebase", "--abort")
        if os.path.exists(SHARED):
            shutil.copy(SHARED, LOCAL + ".conflict")   # 저장소 밖에 둔다(커밋 지저분해짐 방지)
        git(repo, "reset", "--hard", "origin/main")
    git(repo, "push", "--quiet", "origin", "main")


def shared_path():
    """두 PC가 같아야 하는 건 종목 목록뿐이다. 그건 비공개 저장소 claude-memory에 두고,
    창 위치·불투명도처럼 그 PC 사정인 값은 LOCAL(스크립트 옆)에만 남긴다.
    한 파일에 섞으면 창을 옮긴 것만으로 충돌이 나서 목록이 유실된다."""
    if os.environ.get("TICKER_CONFIG"):
        return None                                # 경로를 직접 지정한 PC는 공유 안 함
    repo = os.path.join(os.path.expanduser("~"), "claude-memory")
    if not os.path.isdir(os.path.join(repo, ".git")):
        return None
    os.makedirs(os.path.join(repo, "ticker"), exist_ok=True)
    return os.path.join(repo, "ticker", "ticker_config.json")


LOCAL = os.environ.get("TICKER_CONFIG") or os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "ticker_config.json")
SHARED = shared_path()
DEFAULT = {
    "tickers": ["KOSPI=코스피", "069500=코덱스200", "USDKRW=X=환율",
                "---",
                "BTC-KRW=비트코인", "XRP-KRW=XRP",
                "---",
                "AAPL", "NVDA"],
    "refresh_sec": 5,
    "topmost": True,
    "hidden": False,
    "collapsed": [],                     # 구분선 단위 그룹 접힘 상태
    "pos": None,                         # 직접 옮긴 위치 (없으면 우하단 자동)
    "bg_op": 85,                         # 바탕 불투명도 0~100 (0 = 완전 투명, 100 = 불투명)
    "text_op": 80,                       # 글자 불투명도 0~100 (80 = 지금 상태)
    "bg_color": "#2a2f3a",               # 바탕색 (PALETTE에서 고름)
    "coin_prem": {},                     # 코인별 국내가 ÷ 해외가 (거래소가 막힌 곳에서 쓴다)
    "coin_usd": {},                      # 야후에 원화 페어가 없어 달러로 묻는 코인
}

SPARK = "https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols="
NAVER = "https://polling.finance.naver.com/api/realtime/domestic/{}/{}"
BITHUMB = "https://api.bithumb.com/public/ticker/{}_KRW"     # 종목별. ALL_KRW는 캐시를 타서 값이 멎는다
UPBIT = "https://api.upbit.com/v1/ticker?markets="           # 빗썸이 막힐 때 대체
UPCANDLE = "https://api.upbit.com/v1/candles/minutes/60?count=1&market={}&to={}"
FX = "USDKRW=X"                          # 달러 페어만 있는 코인을 원화로 바꿀 때 쓰는 환율
UA = {"User-Agent": "Mozilla/5.0", "Referer": "https://finance.naver.com/"}
BG, FG, DIM, ACC, LINE = "#2a2f3a", "#f2f5fa", "#9aa5ba", "#7db3ff", "#4d576c"
# 고를 수 있는 바탕색. 글자가 흰 계열이라 어두운 색만 넣는다.
PALETTE = ["#2a2f3a", "#000000", "#16233d", "#2b1f3d", "#14312a"]
KEY = "#010203"                          # 이 색 픽셀은 창에서 아예 뚫린다(윈도우 전용)
TRANSPARENT = False                      # 뚫기가 되는 환경인지 (아래 root 만들고 판단)
ROWLINE = "#353c4a"                      # 행 사이 얇은 선
FIELD = "#1f242e"                        # 설정창 입력칸
THEME = {}                               # 불투명도가 반영된 현재 색
UP, DOWN = "#ff5c66", "#5aa8ff"          # 국내식: 상승 빨강, 하락 파랑
SEP = "---"
RATIO = "/"                              # "XRP-KRW/KAIA-KRW" = 1 XRP가 몇 KAIA인지
ALERT = 5.0                              # 등락률 이 이상이면 반짝임
COIN_SEC = 5                             # 코인 조회 간격(초). 주식과 같은 속도로 본다
COIN_SLOW = 20                           # 거래소가 요청을 끊는 곳(학교망)에서는 이만큼 물러선다
coin_at = [0.0]                          # 마지막 조회 시각
coin_ok = [0.0]                          # 마지막으로 거래소가 응답한 시각
coin_gap = [COIN_SEC]                    # 지금 쓰는 간격 (실패하면 늘고, 되면 다시 줄인다)
coin_src = [""]                          # 코인 값의 출처 (빗썸/업비트/야후) — 화면에 표시
coin_lag = [0.0]                         # 거래소가 준 값이 몇 초 전 것인지 (캐시된 응답 판별)
midnight = {}                            # 코인 -> (날짜, 자정 시가) 하루 한 번만 조회
coin_prem = {}                           # 코인 -> 국내가 ÷ 해외가. 거래소가 막힌 동안 환산에 쓴다
coin_base = {}                           # 코인 -> (날짜, 국내 자정 시가). 등락률 기준을 빗썸과 맞춘다
coin_ext = [True]                        # 야후에 코인도 함께 물어볼지 (코인 탓에 실패하면 끈다)
coin_usd = {}                            # 코인 -> 야후 달러 심볼. 원화 페어가 없는 코인(KAIA 등)
prem_at = [0.0]                          # 환산비를 마지막으로 저장한 시각
PREM_SAVE = 600                          # 환산비 저장 간격(초)
INDEX = {"KOSPI": "KOSPI", "KOSDAQ": "KOSDAQ", "^KS11": "KOSPI", "^KQ11": "KOSDAQ"}
CODE = re.compile(r"^[0-9]{4}[0-9A-Z]{2}$")   # 069500, 0005A0 같은 국내 종목코드
NAME_W = 17                              # 이름 최대 폭(한글 2, 영문 1). 넘으면 잘림
# 국내 ETF 브랜드 약칭 (구 브랜드는 현재 약칭으로 통일). 필요하면 여기에 추가
BRAND = {"KODEX": "K", "TIGER": "T", "RISE": "R", "KBSTAR": "R", "PLUS": "P", "ARIRANG": "P",
         "SOL": "S", "ACE": "A", "KINDEX": "A", "HANARO": "H", "KOSEF": "KF", "TIMEFOLIO": "TF",
         "WOORI": "W", "FOCUS": "F", "UNICORN": "U", "TREX": "TX", "BNK": "B", "히어로즈": "HR",
         "마이티": "MT", "마이다스": "MD", "파워": "PW", "에셋플러스": "EP"}


def split(line):
    """069500=KODEX 200 -> (069500, KODEX 200). USDKRW=X 처럼 심볼에 =가 있어도 안전."""
    sym, sep, name = line.rpartition("=")
    return (sym, name) if sep and len(name) > 1 else (line, line)


def cut(name):
    """브랜드는 약칭으로, 그래도 길면 잘라서 위젯 폭 고정. 한글은 두 칸으로 계산."""
    for full, short in BRAND.items():
        if name.startswith(full + " "):
            name = short + " " + name[len(full) + 1:]
            break
    w = 0
    for i, ch in enumerate(name):
        w += 2 if ord(ch) > 0x2E80 else 1
        if w > NAME_W:
            return name[:i] + "…"
    return name


def mix(a, b, t):
    """색 a와 b를 t 비율로 섞기 (t=0 -> a, t=1 -> b)."""
    t = max(0.0, min(1.0, t))
    ca, cb = (int(a[1:][i:i + 2], 16) for i in (0, 2, 4)), (int(b[1:][i:i + 2], 16) for i in (0, 2, 4))
    return "#%02x%02x%02x" % tuple(round(x + (y - x) * t) for x, y in zip(ca, cb))


def theme():
    """80 = 지금 보이는 상태. 그 아래는 배경에 묻히고, 위로는 더 또렷해짐."""
    t = cfg["text_op"] / 100.0
    for key, base in (("fg", FG), ("dim", DIM), ("up", UP), ("down", DOWN)):
        THEME[key] = mix(BG, base, t / 0.8) if t <= 0.8 else mix(base, "#ffffff", (t - 0.8) * 2.5)
    if TRANSPARENT:
        root.attributes("-alpha", 1.0)                          # 글자 창은 항상 또렷하게
        back.configure(bg=BG)
        back.attributes("-alpha", cfg["bg_op"] / 100.0)         # 바탕만 흐려진다
    else:
        root.attributes("-alpha", cfg["bg_op"] / 100.0)         # 뚫기가 안 되면 예전처럼
    for sym in rows:
        nm, p, c, _ = rows[sym]
        nm.config(fg=THEME["fg"])
        p.config(fg=THEME["fg"])
        if sym in last:
            paint(sym, last[sym])
        else:
            c.config(fg=THEME["dim"])


def repaint(w, old, new):
    """창 안에서 옛 바탕색을 쓰던 것만 새 색으로 바꾼다(입력칸 등 다른 색은 그대로)."""
    try:
        if str(w.cget("bg")) == old:
            w.configure(bg=new)
    except Exception:
        pass
    for child in w.winfo_children():
        repaint(child, old, new)


def set_bg(color, win=None):
    """바탕색 변경. 종목 줄은 만들 때 색이 박히므로 다시 그린다."""
    global BG
    old, BG = BG, color
    cfg["bg_color"] = color
    repaint(root, old, BG)
    if win is not None:
        repaint(win, old, BG)
    build()
    theme()
    save(cfg)


def groups_of(tickers):
    """--- 를 기준으로 그룹 나누기."""
    gs, cur = [], []
    for line in tickers:
        if line.strip().startswith(SEP):
            gs.append(cur)
            cur = []
        else:
            cur.append(line)
    gs.append(cur)
    return [g for g in gs if g]


def visible():
    """접지 않은 그룹의 종목 줄만."""
    gs = groups_of(cfg["tickers"])
    return [x for i, g in enumerate(gs) for x in g if not cfg["collapsed"][i]]


def num(s):
    return float(str(s).replace(",", ""))


def fmt(v, dec):
    return f"{v:,.{dec}f}"


def get(url):
    return json.load(urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=10))


def naver(kind, syms):
    """국내 실시간. kind: stock(종목코드) 또는 index(KOSPI/KOSDAQ). 한 번에 여러 개."""
    out = {}
    for d in get(NAVER.format(kind, ",".join(syms)))["datas"]:
        price, diff = num(d["closePrice"]), num(d["compareToPreviousClosePrice"])
        out[d["itemCode"]] = (price, diff, num(d["fluctuationsRatio"]), 0, d.get("stockName"))
    return out


def bithumb(syms):
    """국내 코인 시세(빗썸). 야후는 해외 평균가라 국내 거래소와 차이가 큼.
    ⚠️ 등락률 기준은 거래소마다 다르다 — 빗썸 화면은 자정, 업비트 화면은 09시,
    '24시간 전 대비'는 또 다른 값이다(2026-08-21 XRP: 13.4 / 9.2 / 20.3%).
    보고 있는 화면과 같아야 하므로 빗썸 '변동(당일)'과 같은 prev_closing_price(전일 종가)를
    기준으로 쓴다. opening_price(자정 시가)는 한 틱 뒤라 미묘하게 어긋난다 — 2026-08-29
    KAIA에서 0.21%p(-7.35 vs 화면 -7.14) 벌어졌고, 교환비율 줄에서 더 증폭됐다."""
    out, lag = {}, 0.0
    for s in syms:
        v = get(BITHUMB.format(s.split("-")[0]))["data"]
        price, prev = float(v["closing_price"]), float(v["prev_closing_price"])
        ts = float(v.get("date") or 0) / 1000.0   # 응답에 박힌 거래소 시각
        lag = max(lag, time.time() - ts if ts else 0.0)
        diff = price - prev
        out[s] = (price, diff, diff / prev * 100 if prev else 0.0,
                  0 if price >= 100 else 2, None)
    coin_lag[0] = max(0.0, lag)
    return out


def kst_day():
    """오늘 날짜(KST). 코인 등락률 기준인 자정이 언제인지 판단할 때 쓴다."""
    return datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=9))).strftime("%Y%m%d")


def kst_midnight_open(market):
    """오늘 00시(KST) 시가. 빗썸 화면과 같은 기준으로 맞추기 위함. 하루 한 번만 조회."""
    now = datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=9)))
    day = now.strftime("%Y%m%d")
    if midnight.get(market, ("",))[0] != day:
        to = (now.replace(hour=0, minute=0, second=0, microsecond=0)
              + datetime.timedelta(hours=1)).astimezone(datetime.timezone.utc)
        try:
            c = get(UPCANDLE.format(market, to.strftime("%Y-%m-%dT%H:%M:%S")))
            midnight[market] = (day, c[0]["opening_price"])
        except Exception:
            return None
    return midnight[market][1]


def upbit(syms):
    """빗썸이 막혔을 때 쓰는 대체 소스. 업비트 화면 기준(09시)이 아니라 빗썸에 가깝게
    자정 시가를 기준으로 계산한다(빗썸의 전일 종가와는 한 틱 차이)."""
    markets = ",".join("KRW-" + x.split("-")[0] for x in syms)
    out = {}
    for d in get(UPBIT + markets):
        price = d["trade_price"]
        prev = kst_midnight_open(d["market"]) or d["prev_closing_price"]
        diff = price - prev
        out[d["market"].split("-")[1] + "-KRW"] = (
            price, diff, diff / prev * 100 if prev else 0.0, 0 if price >= 100 else 2, None)
    return out


def yahoo(syms):
    """해외·코인·환율. 한 번의 요청으로 전 종목."""
    out = {}
    for r in get(SPARK + urllib.parse.quote(",".join(syms)))["spark"]["result"]:
        m = r["response"][0]["meta"]
        price = m["regularMarketPrice"]
        prev = m.get("chartPreviousClose") or price
        diff = price - prev
        # 원화 표시 자산은 정수, 환율과 달러 종목은 소수 둘째 자리까지
        dec = 2 if r["symbol"].endswith("=X") or m.get("currency") != "KRW" else 0
        out[r["symbol"]] = (price, diff, diff / prev * 100 if prev else 0.0, dec, None)
    return out


def convert(sym, far):
    """국내 거래소가 막혔을 때, 해외 시세(far)에 마지막으로 잰 비율을 곱해 국내가를 추정한다.
    등락률은 국내 자정 시가(빗썸 화면 기준)를 알고 있으면 그걸 쓰고, 없으면 해외 기준."""
    r = coin_prem.get(sym) or 1.0
    price = far[0] * r
    day, prev = coin_base.get(sym) or ("", 0)
    if day != kst_day() or not prev:
        prev = (far[0] - far[1]) * r
    diff = price - prev
    return (price, diff, diff / prev * 100 if prev else 0.0,
            0 if price >= 100 else 2, None)


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return {}


def write(path, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def load():
    """스크립트 옆 파일(그 PC 설정) 위에 공용 저장소의 종목 목록을 덮어쓴다."""
    mine = read(LOCAL)
    cfg = {**DEFAULT, **mine}
    if mine and "bg_color" not in mine:
        # 옛 눈금은 20~100이 알파 0.76~1.0이었다. 보이던 밝기 그대로 새 눈금으로 옮긴다.
        cfg["bg_op"] = round(70 + cfg["bg_op"] * 0.3)
    if SHARED:
        sync()                                     # 다른 PC가 바꾼 목록을 먼저 받아온다
        tickers = read(SHARED).get("tickers")
        if tickers:
            cfg["tickers"] = tickers
        else:
            write(SHARED, {"tickers": cfg["tickers"]})   # 공용이 비어 있으면 내 것으로 시작
            sync()
    return cfg


def save(cfg):
    write(LOCAL, cfg)
    # 목록을 고쳤을 때만 다른 PC로 올린다. 창 위치·접힘·불투명도는 그 PC에만 남는다.
    global pushed
    if SHARED and cfg["tickers"] != pushed:
        pushed = list(cfg["tickers"])
        write(SHARED, {"tickers": pushed})
        Thread(target=sync, daemon=True).start()


cfg = load()
pushed = list(cfg["tickers"])            # 마지막으로 올린 종목 목록
coin_prem.update(cfg.get("coin_prem") or {})
coin_usd.update(cfg.get("coin_usd") or {})
BG = cfg.get("bg_color") or BG           # 저장해 둔 바탕색으로 시작
rows = {}
alerts = {}                              # 반짝일 종목 -> 색
last = {}                                # 마지막 시세 (색만 다시 칠할 때 사용)
caps = []                                # 그룹 접기 버튼 (평소 숨김, 마우스 올리면 표시)
timer = None

root = tk.Tk()
root.overrideredirect(True)
root.attributes("-topmost", cfg["topmost"])
root.geometry("+40+40")

# 바탕과 글자를 창 두 개로 나눈다. Tk의 투명도(-alpha)는 창 전체에 걸려서 한 창에 두면
# 바탕을 흐리게 할 때 글자까지 같이 사라진다. 그래서 바탕은 back 창이 alpha로 그리고,
# 글자 창(root)은 바탕색 픽셀만 뚫어(-transparentcolor) 글자는 항상 또렷하게 남긴다.
back = tk.Toplevel(root)
back.overrideredirect(True)
back.attributes("-topmost", cfg["topmost"])
back.configure(bg=BG)
# ⛔ 2026-08-25: 색상키 투명(-transparentcolor)은 이 환경에서 글자 잔상이 남아 화면이
# 뭉개졌다(사용자 스크린샷 확인). 창 전체 투명도로 되돌린다 — 바탕을 흐리게 하면 글자도
# 같이 흐려지지만, 적어도 읽을 수는 있다. 다시 시도하려면 아래 두 줄을 살리면 된다.
#     root.attributes("-transparentcolor", KEY); TRANSPARENT = True
back.withdraw()


def panel_bg():
    """시세창 안쪽 배경색. 뚫기가 되면 KEY(투명), 아니면 그냥 바탕색."""
    return KEY if TRANSPARENT else BG


root.configure(bg=panel_bg())
head = tk.Frame(root, bg=panel_bg())
head.pack(fill="x", padx=6, pady=(2, 0))
status = tk.Label(head, text="", bg=panel_bg(), fg=ACC, font=("Malgun Gothic", 7))
status.pack(side="left", padx=6)
gear = tk.Label(head, text="⚙", bg=panel_bg(), fg=ACC, font=("Segoe UI Symbol", 9), cursor="hand2")
gear.pack(side="right", padx=(8, 0))
body = tk.Frame(root, bg=panel_bg())
body.pack(fill="both", padx=6, pady=(0, 4))


def sync_back():
    """배경 창을 시세창과 같은 자리·같은 크기로 두고, 글자 창을 그 위로 올린다."""
    if not TRANSPARENT:
        return
    root.update_idletasks()
    w = root.winfo_width() or root.winfo_reqwidth()
    h = root.winfo_height() or root.winfo_reqheight()
    back.geometry("%dx%d+%d+%d" % (w, h, root.winfo_x(), root.winfo_y()))
    root.lift(back)


def build():
    for w in body.winfo_children():
        w.destroy()
    rows.clear()
    alerts.clear()
    caps.clear()
    gs = groups_of(cfg["tickers"])
    col = list(cfg.get("collapsed") or [])
    cfg["collapsed"] = (col + [False] * len(gs))[:len(gs)]
    r = 0
    for gi, group in enumerate(gs):
        shut = cfg["collapsed"][gi]
        strip = tk.Frame(body, bg=panel_bg())
        strip.grid(row=r, column=0, columnspan=3, sticky="ew")
        r += 1
        bar = tk.Frame(strip, bg=LINE, height=1)
        bar.pack(fill="x", expand=True, pady=4)
        # place로 띄워서 창 크기에 영향 없음. 평소엔 숨김, 마우스 올리면 왼쪽에 나타남
        cap = tk.Label(strip, text="▸%d" % len(group) if shut else "▾", bg=panel_bg(), fg=ACC,
                       font=("Segoe UI Symbol", 7), cursor="hand2")
        caps.append(cap)
        for w in (strip, cap, bar):
            w.bind("<Button-1>", lambda e, i=gi: fold(i))
        if shut:
            continue
        for line in group:
            sym, name = split(line)
            given = name != sym                  # 직접 지정한 이름은 약칭 처리 안 함
            nm = tk.Label(body, text=name if given else cut(name), bg=panel_bg(), fg=FG,
                          font=("Malgun Gothic", 8, "bold"))
            nm.grid(row=r, column=0, sticky="w", padx=(0, 2))
            p = tk.Label(body, text="…", bg=panel_bg(), fg=FG, font=("Consolas", 9))
            c = tk.Label(body, text="", bg=panel_bg(), fg=DIM, font=("Consolas", 8))
            p.grid(row=r, column=1, sticky="e")
            c.grid(row=r, column=2, sticky="e", padx=(3, 0))
            rows[sym] = (nm, p, c, given)
            tk.Frame(body, bg=ROWLINE, height=1).grid(row=r + 1, column=0, columnspan=3,
                                                      sticky="ew")
            r += 2
    theme()
    sync_back()                          # 줄 수가 바뀌면 배경 창 크기도 맞춘다


def hover(show):
    for cap in caps:
        if show:
            cap.place(x=0, rely=0.5, anchor="w")
        else:
            cap.place_forget()


def maybe_hide():
    x, y = root.winfo_pointerxy()
    inside = (root.winfo_rootx() <= x <= root.winfo_rootx() + root.winfo_width()
              and root.winfo_rooty() <= y <= root.winfo_rooty() + root.winfo_height())
    hover(inside)


def fold(i):
    """구분선 클릭 -> 그 그룹 접기/펴기. 접힌 그룹은 시세도 안 불러옴."""
    cfg["collapsed"][i] = not cfg["collapsed"][i]
    save(cfg)
    build()
    refresh()
    place_panel()
    root.after(1500, place_panel)        # 시세가 채워져 크기가 바뀌면 다시 붙임


def paint(sym, res):
    if sym not in rows:
        return
    nm, p, c, given = rows[sym]
    if res is None:
        c.config(text="err", fg=DIM)
        return
    last[sym] = res
    price, diff, pct, dec, name = res
    if name and not given and nm.cget("text") == sym:   # 코드만 넣었으면 종목명 자동 표시
        nm.config(text=cut(name))
    if abs(pct) >= ALERT:
        alerts[sym] = THEME["up"] if diff > 0 else THEME["down"]
    elif alerts.pop(sym, None):
        c.config(bg=panel_bg())
    text = fmt(price, dec)
    p.config(text=text, font=("Consolas", 7 if len(text) > 9 else 9))   # 자릿수 많으면 축소
    c.config(text=f"{pct:+.2f}%",
             fg=THEME["up"] if diff > 0 else THEME["down"] if diff < 0 else THEME["dim"])


def blink(on=[False]):
    on[0] = not on[0]
    flash = next(iter(alerts.values()), None)          # 숨겨둬도 탭 버튼이 알려줌
    tab_btn.config(bg=flash if flash and on[0] else ACC,
                   fg="#e8ecf3" if flash and on[0] else "#0b0e14")
    for sym, color in list(alerts.items()):
        if sym in rows:
            c = rows[sym][2]
            c.config(bg=color if on[0] else panel_bg(), fg="#12151c" if on[0] else color)
    root.after(500, blink)


def schedule(sec):
    global timer
    if timer:
        root.after_cancel(timer)
    timer = root.after(int(sec * 1000), refresh)


def refresh():
    def work():
        keys = [split(x)[0] for x in visible()]
        # "A/B" 는 교환비율 줄이다(1 A = 몇 B). 화면엔 비율만 뜨지만 시세는 둘 다 받아야 한다.
        need = []
        for k in keys:
            for one in (k.split("/") if RATIO in k else [k]):
                if one not in need:
                    need.append(one)
        groups = {"index": [], "stock": [], "coin": [], "yahoo": []}
        for k in need:
            groups["index" if k in INDEX else "stock" if CODE.match(k)
                   else "coin" if k.endswith("-KRW") else "yahoo"].append(k)

        data, wait = {}, 0
        for kind in ("index", "stock"):
            syms = groups[kind]
            if syms:
                try:
                    got = naver(kind, [INDEX.get(s, s) for s in syms])
                    data.update({s: got[INDEX.get(s, s)] for s in syms if INDEX.get(s, s) in got})
                except Exception:
                    pass
        if groups["coin"] and time.time() - coin_at[0] >= coin_gap[0] - 0.5:  # 0.5초는 흔들림 여유
            coin_at[0] = time.time()
            for source in (bithumb, upbit):          # 빗썸이 막히면 업비트로
                try:
                    data.update(source(groups["coin"]))
                    coin_ok[0] = time.time()
                    coin_gap[0] = COIN_SEC           # 잘 받아지면 계속 빠르게
                    coin_src[0] = "빗썸" if source is bithumb else "업비트"
                    break
                except Exception:
                    continue
            else:
                coin_gap[0] = COIN_SLOW              # 둘 다 막히면 물러섰다가 다시 시도
                coin_src[0] = "거래소 막힘"

        # 야후에는 코인도 같이 물어본다(요청은 그대로 한 번). 거래소가 살아 있을 땐 국내가와의
        # 비율을 재 두고, 막히면 그 비율로 해외 시세를 국내가로 환산해서 쓴다.
        # 야후에 원화 페어가 없는 코인(KAIA 등)은 달러 페어로 묻고 환율을 곱해 원화로 만든다.
        ext, ylist = {}, list(groups["yahoo"])
        if coin_ext[0]:
            for k in groups["coin"]:
                for sym in (coin_usd[k], FX) if k in coin_usd else (k,):
                    if sym not in ylist:
                        ylist.append(sym)
        if ylist:
            try:
                ext = yahoo(ylist)
            except urllib.error.HTTPError as e:
                wait = 60 if e.code == 429 else 0    # 요청 과다 -> 1분 쉬었다 재시도
            except Exception:
                try:                                 # 코인 심볼 탓이면 주식만이라도 받는다
                    ext = yahoo(groups["yahoo"]) if groups["yahoo"] else {}
                    coin_ext[0] = False
                except Exception:
                    pass
        data.update({k: v for k, v in ext.items() if k not in groups["coin"]})

        # 코인의 해외 시세를 원화 기준으로 모은다. 원화 페어가 없으면 달러 페어 x 환율.
        far, fx = {}, ext.get(FX)
        for k in groups["coin"]:
            if k in ext:
                far[k] = ext[k]
            elif k in coin_usd and fx and ext.get(coin_usd[k]):
                usd, diff, pct = ext[coin_usd[k]][:3]
                # 등락률은 달러 기준 그대로 둔다 — 환율 변동이 안 섞인 코인 자체의 움직임이다.
                won = usd * fx[0]
                far[k] = (won, diff * fx[0], pct, 0 if won >= 100 else 2, None)
            elif ext and coin_ext[0] and k not in coin_usd:
                coin_usd[k] = k.split("-")[0] + "-USD"   # 원화 페어가 없다 -> 다음 조회부터 달러로

        blocked = time.time() - coin_ok[0] > 30      # 한두 번 걸러도 직전 값이 아직 쓸 만하다
        conv, raw = [], []                           # 환산한 코인 / 그중 환산비가 없어 해외 원값인 것
        for k in groups["coin"]:
            if k in data:                            # 거래소에서 받았다 -> 해외와의 비율을 기억
                if far.get(k) and far[k][0]:
                    coin_prem[k] = data[k][0] / far[k][0]
                    coin_base[k] = (kst_day(), data[k][0] - data[k][1])
            elif far.get(k) and blocked:
                data[k] = convert(k, far[k])
                conv.append(k)
                if k not in coin_prem:
                    raw.append(k)
        if conv:                                     # 일부만 원값일 수 있으므로 뭉뚱그리지 않는다
            coin_src[0] = ("해외평균(환산비 없음)" if len(raw) == len(conv) else
                           "해외 환산 · %s 원값" % "/".join(x.split("-")[0] for x in raw) if raw else
                           "해외 환산")
        if (coin_prem or coin_usd) and time.time() - prem_at[0] > PREM_SAVE:
            prem_at[0] = time.time()
            cfg["coin_prem"] = {k: round(v, 6) for k, v in coin_prem.items()}
            cfg["coin_usd"] = dict(coin_usd)
            root.after(0, save, cfg)

        note = ("요청 과다 · 1분 대기" if wait else
                "코인: 빗썸 %d초 전 값" % coin_lag[0] if coin_src[0] == "빗썸" and coin_lag[0] > 15 else
                "" if coin_src[0] in ("빗썸", "") else "코인: " + coin_src[0])
        root.after(0, status.config, {"text": note})
        for k in keys:                        # 교환비율 줄 계산 (1 A = 몇 B)
            if RATIO in k:
                a, b = k.split(RATIO, 1)
                if a in data and b in data and data[b][0]:
                    pa, da = data[a][0], data[a][1]
                    pb, db = data[b][0], data[b][1]
                    now = pa / pb
                    was = (pa - da) / (pb - db) if (pb - db) else 0
                    gap = now - was
                    data[k] = (now, gap, gap / was * 100 if was else 0.0,
                               2 if now < 100 else 0, None)
        for k in keys:
            if k in data or k not in last:    # 못 받아온 건 직전 값 그대로 둠
                root.after(0, paint, k, data.get(k))
        root.after(0, schedule, wait or cfg["refresh_sec"])

    Thread(target=work, daemon=True).start()


def settings(_=None):
    win = tk.Toplevel(root, bg=BG)
    win.title("설정")
    win.attributes("-topmost", True)
    tk.Label(win, text="목록 (한 줄에 하나 · 심볼=이름 · --- 는 구분선)", bg=BG, fg=DIM,
             font=("Malgun Gothic", 9)).pack(anchor="w", padx=12, pady=(12, 4))
    txt = tk.Text(win, width=30, height=12, bg=FIELD, fg=FG, insertbackground=FG,
                  relief="flat", font=("Consolas", 11))
    txt.insert("1.0", "\n".join(cfg["tickers"]))
    txt.pack(padx=12)
    tip = ("국내: 6자리 코드만 (069500, 0005A0) · 지수 KOSPI KOSDAQ\n"
           "환율 USDKRW=X · 코인 BTC-KRW · 미국 AAPL\n"
           "교환비율: XRP-KRW/KAIA-KRW=XRP당카이아\n"
           "이름이 길면 069500=코덱스200 처럼 직접 지정")
    tk.Label(win, text=tip, bg=BG, fg=DIM, justify="left",
             font=("Malgun Gothic", 8)).pack(anchor="w", padx=12, pady=(4, 6))
    top = tk.BooleanVar(value=cfg["topmost"])
    tk.Checkbutton(win, text="항상 위에 표시", variable=top, bg=BG, fg=FG, selectcolor=FIELD,
                   activebackground=BG, activeforeground=FG, bd=0, highlightthickness=0,
                   font=("Malgun Gothic", 9)).pack(anchor="w", padx=8)
    was = (cfg["bg_op"], cfg["text_op"], cfg["bg_color"])

    def slider(label, key, lo):
        row = tk.Frame(win, bg=BG)
        row.pack(fill="x", padx=10, pady=(2, 0))
        tk.Label(row, text=label, bg=BG, fg=DIM, width=8, anchor="w",
                 font=("Malgun Gothic", 9)).pack(side="left")
        sc = tk.Scale(row, from_=lo, to=100, orient="horizontal", bg=BG, fg=FG,
                      troughcolor=FIELD, activebackground=ACC, highlightthickness=0,
                      bd=0, sliderrelief="flat", length=150, font=("Consolas", 7),
                      command=lambda v: (cfg.update({key: int(float(v))}), theme()))
        sc.set(cfg[key])
        sc.pack(side="left")

        # 눌러서 끌면 그 자리로 — Tk 기본은 홈만 잡아야 끌리고 여백은 한 칸씩 움직인다
        def drag(e):
            span = max(1, sc.winfo_width() - 14)
            sc.set(max(lo, min(100, round(lo + (e.x - 7) / span * (100 - lo)))))
            return "break"
        sc.bind("<Button-1>", drag)
        sc.bind("<B1-Motion>", drag)
        return sc

    slider("바탕 투명", "bg_op", 0)          # 0 = 완전 투명
    slider("글자 진하기", "text_op", 20)

    crow = tk.Frame(win, bg=BG)
    crow.pack(fill="x", padx=10, pady=(6, 2))
    tk.Label(crow, text="바탕색", bg=BG, fg=DIM, width=8, anchor="w",
             font=("Malgun Gothic", 9)).pack(side="left")
    chips = []

    def pick(color):
        set_bg(color, win)
        for chip, c in chips:               # repaint가 옛 바탕색 칩까지 바꾸므로 되돌린다
            chip.config(bg=c, highlightbackground=ACC if c == color else FIELD)

    for c in PALETTE:
        chip = tk.Label(crow, bg=c, width=3, height=1, cursor="hand2",
                        highlightthickness=2, highlightbackground=ACC if c == BG else FIELD)
        chip.pack(side="left", padx=3)
        chip.bind("<Button-1>", lambda e, c=c: pick(c))
        chips.append((chip, c))

    # 어느 파일이 도는지·언제 코드인지 한눈에. (2026-08-24: 저장소는 최신인데 옛 드라이브
    # 사본이 돌고 있어서 "왜 안 바뀌냐"로 한나절을 썼다. 눈으로 보이면 그럴 일이 없다.)
    me = os.path.abspath(__file__)
    stamp = time.strftime("%m-%d %H:%M", time.localtime(os.path.getmtime(me)))
    tk.Label(win, text="코드 %s · %s\n%s" % (stamp, update_note[0], me), bg=BG, fg=DIM,
             justify="left", font=("Malgun Gothic", 7)).pack(anchor="w", padx=12, pady=(6, 0))

    bar = tk.Frame(win, bg=BG)
    bar.pack(fill="x", padx=12, pady=(4, 12))
    tk.Label(bar, text="갱신(초)", bg=BG, fg=DIM, font=("Malgun Gothic", 9)).pack(side="left")
    sec = tk.Entry(bar, width=5, bg=FIELD, fg=FG, insertbackground=FG, relief="flat",
                   justify="center", font=("Consolas", 11))
    sec.insert(0, str(cfg["refresh_sec"]))
    sec.pack(side="left", padx=6)
    err = tk.Label(win, text="", bg=BG, fg=DOWN, font=("Malgun Gothic", 8))
    err.pack(padx=12)

    def apply():
        tickers = [s.strip() for s in txt.get("1.0", "end").split("\n") if s.strip()]
        if not any(not t.startswith(SEP) for t in tickers):
            err.config(text="종목을 하나 이상 입력하세요")
            return
        try:
            n = max(3, int(sec.get()))          # 3초 미만은 야후가 차단
        except ValueError:
            err.config(text="갱신 주기는 숫자로")
            return
        cfg.update(tickers=tickers, refresh_sec=n, topmost=top.get())
        save(cfg)
        root.attributes("-topmost", cfg["topmost"])
        build()
        refresh()
        win.destroy()

    tk.Button(bar, text="저장", command=apply, bg=ACC, fg="#12151c", relief="flat",
              font=("Malgun Gothic", 9, "bold"), padx=14, cursor="hand2").pack(side="right")
    def cancel():
        cfg.update(bg_op=was[0], text_op=was[1])
        if cfg["bg_color"] != was[2]:
            set_bg(was[2], win)
        theme()
        win.destroy()

    win.protocol("WM_DELETE_WINDOW", cancel)
    tk.Button(bar, text="취소", command=cancel, bg=FIELD, fg=FG, relief="flat",
              font=("Malgun Gothic", 9), padx=10, cursor="hand2").pack(side="right", padx=6)
    place_settings(win)


tab = tk.Toplevel(root, bg=ACC)
tab.overrideredirect(True)
tab.attributes("-topmost", True)          # 패널을 숨겨도 이 버튼은 항상 보이게
tab_btn = tk.Label(tab, text="◀", bg=ACC, fg="#12151c", font=("Segoe UI Symbol", 10, "bold"),
                   padx=6, pady=2, cursor="hand2")
tab_btn.pack()
tab.update_idletasks()
tab.geometry("+%d+%d" % (tab.winfo_screenwidth() - tab.winfo_reqwidth() - 12,
                         tab.winfo_screenheight() - tab.winfo_reqheight() - 60))


MARGIN = 4
SNAP = 24                                # 화면 가장자리 자석 범위(px)


class MONITORINFO(ctypes.Structure):
    _fields_ = [("cbSize", ctypes.wintypes.DWORD), ("rcMonitor", ctypes.wintypes.RECT),
                ("rcWork", ctypes.wintypes.RECT), ("dwFlags", ctypes.wintypes.DWORD)]


def work_area(x=0, y=0):
    """(x, y)가 놓인 모니터의 작업 영역 (왼쪽, 위, 오른쪽, 아래).
    ⚠️ SPI_GETWORKAREA 는 주 모니터만 알려줘서, 그걸 쓰면 보조 모니터로 옮겨도
    놓는 순간 주 모니터 안으로 되돌아온다. 모니터별로 물어봐야 한다."""
    try:
        u = ctypes.windll.user32
        u.MonitorFromPoint.argtypes = [ctypes.wintypes.POINT, ctypes.wintypes.DWORD]
        u.MonitorFromPoint.restype = ctypes.c_void_p          # 64비트 핸들이 잘리지 않게
        u.GetMonitorInfoW.argtypes = [ctypes.c_void_p, ctypes.POINTER(MONITORINFO)]
        mi = MONITORINFO()
        mi.cbSize = ctypes.sizeof(MONITORINFO)
        pt = ctypes.wintypes.POINT(int(x), int(y))
        u.GetMonitorInfoW(u.MonitorFromPoint(pt, 2), ctypes.byref(mi))   # 2 = 가장 가까운 모니터
        r = mi.rcWork
        return r.left, r.top, r.right, r.bottom
    except Exception:
        return 0, 0, root.winfo_screenwidth(), root.winfo_screenheight()


def place_settings(win):
    """설정창이 시세창을 가리지 않게 옆에 붙인다. 왼쪽 → 오른쪽 → 위 순으로 자리를 본다."""
    win.update_idletasks()
    anchor = tab if cfg.get("hidden") else root   # 숨긴 상태면 작은 버튼을 기준으로
    anchor.update_idletasks()
    w, h = win.winfo_reqwidth(), win.winfo_reqheight()
    px, py = anchor.winfo_x(), anchor.winfo_y()
    pw = anchor.winfo_width() or anchor.winfo_reqwidth()
    ph = anchor.winfo_height() or anchor.winfo_reqheight()
    left, top, right, bottom = work_area(px + pw // 2, py + ph // 2)
    gap = 6
    if px - gap - w >= left:                      # 왼쪽에 자리가 있으면 왼쪽
        x = px - gap - w
    elif px + pw + gap + w <= right:              # 없으면 오른쪽
        x = px + pw + gap
    else:                                         # 양옆이 안 되면 위(모자라면 아래)
        x = min(max(px, left), max(left, right - w))
        y = py - gap - h
        win.geometry("+%d+%d" % (x, max(top, y if y >= top else min(bottom - h, py + ph + gap))))
        return
    win.geometry("+%d+%d" % (x, min(max(py + ph - h, top), max(top, bottom - h))))


def place_panel():
    """저장된 위치(없으면 우하단)에 시세창을 놓고, 버튼을 그 위에 붙임."""
    root.update_idletasks()
    tab.update_idletasks()
    pw, ph = root.winfo_reqwidth(), root.winfo_reqheight()
    tw, th = tab.winfo_reqwidth(), tab.winfo_reqheight()
    if cfg.get("pos"):
        px, py = cfg["pos"]
    else:
        left, top, right, bottom = work_area()            # 처음엔 주 모니터 우하단
        px, py = right - pw - MARGIN, bottom - ph - MARGIN
    # 클램프는 창이 놓인 모니터 기준으로 — 그래야 보조 모니터 위치가 유지된다
    left, top, right, bottom = work_area(px + pw // 2, py + ph // 2)
    px = min(max(px, left), max(left, right - pw))
    py = min(max(py, top), max(top, bottom - ph))
    root.geometry("+%d+%d" % (px, py))
    ty = py - th - 2
    if ty < top:                          # 위쪽에 붙였으면 버튼을 아래로
        ty = py + ph + 2
    tab.geometry("+%d+%d" % (px + pw - tw, ty))
    sync_back()


def toggle(_=None):
    cfg["hidden"] = not cfg["hidden"]
    if cfg["hidden"]:
        root.withdraw()
        back.withdraw()
    else:
        root.deiconify()
        if TRANSPARENT:
            back.deiconify()
    place_panel()
    tab_btn.config(text="◀" if cfg["hidden"] else "▶")
    save(cfg)


tab_btn.bind("<Button-1>", toggle)

root.bind("<Enter>", lambda e: hover(True))
root.bind("<Leave>", lambda e: root.after(120, maybe_hide))

drag = {}


def move(e):
    left, top, right, bottom = work_area(e.x_root, e.y_root)   # 커서가 있는 모니터에 자석
    w, h = root.winfo_width(), root.winfo_height()
    x, y = e.x_root - drag["x"], e.y_root - drag["y"]
    if abs(x - left) < SNAP:
        x = left
    if abs(x + w - right) < SNAP:
        x = right - w
    if abs(y - top) < SNAP:
        y = top
    if abs(y + h - bottom) < SNAP:
        y = bottom - h
    root.geometry("+%d+%d" % (x, y))
    if TRANSPARENT:
        back.geometry("+%d+%d" % (x, y))


def drop(_=None):
    cfg["pos"] = [root.winfo_x(), root.winfo_y()]
    save(cfg)
    place_panel()                         # 버튼도 따라오게


grab = lambda e: drag.update(x=e.x_root - root.winfo_x(), y=e.y_root - root.winfo_y())
root.bind("<Button-1>", grab)
root.bind("<B1-Motion>", move)
root.bind("<ButtonRelease-1>", drop)
# 뚫린 부분은 클릭이 배경 창으로 내려간다 — 거기서도 끌기·우클릭·펼침이 되어야 한다
back.bind("<Button-1>", grab)
back.bind("<B1-Motion>", move)
back.bind("<ButtonRelease-1>", drop)
back.bind("<Enter>", lambda e: hover(True))
back.bind("<Leave>", lambda e: root.after(120, maybe_hide))
gear.bind("<Button-1>", lambda e: (settings(), "break")[1])
menu = tk.Menu(root, tearoff=0)
menu.add_command(label="새로고침", command=refresh)
menu.add_command(label="설정", command=settings)
menu.add_command(label="숨기기/보이기", command=toggle)
menu.add_command(label="위치 초기화", command=lambda: (cfg.update(pos=None), save(cfg), place_panel()))
menu.add_command(label="닫기", command=root.destroy)
root.bind("<Button-3>", lambda e: menu.tk_popup(e.x_root, e.y_root))
tab_btn.bind("<Button-3>", lambda e: menu.tk_popup(e.x_root, e.y_root))
back.bind("<Button-3>", lambda e: menu.tk_popup(e.x_root, e.y_root))
# 바탕을 클릭하면 윈도우가 바탕 창을 앞으로 올려 글자가 덮인다. 누를 때마다 다시 올려준다.
for _ev in ("<Button-1>", "<ButtonRelease-1>", "<Button-3>"):
    back.bind(_ev, lambda e: sync_back(), add="+")

build()
refresh()
blink()
if cfg["hidden"]:
    root.withdraw()
tab_btn.config(text="◀" if cfg["hidden"] else "▶")
place_panel()
root.after(2000, place_panel)             # 시세가 채워지면 크기가 바뀌므로 한 번 더
root.mainloop()
