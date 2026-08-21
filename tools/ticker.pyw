# 바탕화면 실시간 시세 위젯. 드래그로 이동, 톱니로 설정.
# 목록 한 줄에 하나: "심볼" 또는 "심볼=표시이름", "---" 는 구분선.
# 국내(6자리 코드, KOSPI/KOSDAQ)는 네이버 실시간, 해외/코인/환율은 야후.
import ctypes, ctypes.wintypes, json, os, re, tkinter as tk
import urllib.error, urllib.parse, urllib.request
from threading import Thread

CONFIG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ticker_config.json")
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
}

SPARK = "https://query1.finance.yahoo.com/v7/finance/spark?range=1d&interval=1d&symbols="
NAVER = "https://polling.finance.naver.com/api/realtime/domestic/{}/{}"
UA = {"User-Agent": "Mozilla/5.0", "Referer": "https://finance.naver.com/"}
BG, FG, DIM, ACC, LINE = "#2a2f3a", "#f2f5fa", "#9aa5ba", "#7db3ff", "#4d576c"
ROWLINE = "#353c4a"                      # 행 사이 얇은 선
FIELD = "#1f242e"                        # 설정창 입력칸
ALPHA = 0.94                             # 창 불투명도 (1.0 = 불투명)
UP, DOWN = "#ff5c66", "#5aa8ff"          # 국내식: 상승 빨강, 하락 파랑
SEP = "---"
ALERT = 5.0                              # 등락률 이 이상이면 반짝임
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


def load():
    try:
        with open(CONFIG, encoding="utf-8") as f:
            return {**DEFAULT, **json.load(f)}
    except Exception:
        return json.loads(json.dumps(DEFAULT))


def save(cfg):
    with open(CONFIG, "w", encoding="utf-8") as f:
        json.dump(cfg, f, ensure_ascii=False, indent=2)


cfg = load()
rows = {}
alerts = {}                              # 반짝일 종목 -> 색
caps = []                                # 그룹 접기 버튼 (평소 숨김, 마우스 올리면 표시)
timer = None

root = tk.Tk()
root.overrideredirect(True)
root.attributes("-topmost", cfg["topmost"])
root.configure(bg=BG)
root.attributes("-alpha", ALPHA)
root.geometry("+40+40")

head = tk.Frame(root, bg=BG)
head.pack(fill="x", padx=6, pady=(2, 0))
status = tk.Label(head, text="", bg=BG, fg=ACC, font=("Malgun Gothic", 7))
status.pack(side="left", padx=6)
gear = tk.Label(head, text="⚙", bg=BG, fg=ACC, font=("Segoe UI Symbol", 9), cursor="hand2")
gear.pack(side="right", padx=(8, 0))
body = tk.Frame(root, bg=BG)
body.pack(fill="both", padx=6, pady=(0, 4))


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
        strip = tk.Frame(body, bg=BG)
        strip.grid(row=r, column=0, columnspan=3, sticky="ew")
        r += 1
        bar = tk.Frame(strip, bg=LINE, height=1)
        bar.pack(fill="x", expand=True, pady=4)
        # place로 띄워서 창 크기에 영향 없음. 평소엔 숨김, 마우스 올리면 왼쪽에 나타남
        cap = tk.Label(strip, text="▸%d" % len(group) if shut else "▾", bg=BG, fg=ACC,
                       font=("Segoe UI Symbol", 7), cursor="hand2")
        caps.append(cap)
        for w in (strip, cap, bar):
            w.bind("<Button-1>", lambda e, i=gi: fold(i))
        if shut:
            continue
        for line in group:
            sym, name = split(line)
            given = name != sym                  # 직접 지정한 이름은 약칭 처리 안 함
            nm = tk.Label(body, text=name if given else cut(name), bg=BG, fg=FG,
                          font=("Malgun Gothic", 8, "bold"))
            nm.grid(row=r, column=0, sticky="w", padx=(0, 2))
            p = tk.Label(body, text="…", bg=BG, fg=FG, font=("Consolas", 9))
            c = tk.Label(body, text="", bg=BG, fg=DIM, font=("Consolas", 8))
            p.grid(row=r, column=1, sticky="e")
            c.grid(row=r, column=2, sticky="e", padx=(3, 0))
            rows[sym] = (nm, p, c, given)
            tk.Frame(body, bg=ROWLINE, height=1).grid(row=r + 1, column=0, columnspan=3,
                                                      sticky="ew")
            r += 2


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
    price, diff, pct, dec, name = res
    if name and not given and nm.cget("text") == sym:   # 코드만 넣었으면 종목명 자동 표시
        nm.config(text=cut(name))
    if abs(pct) >= ALERT:
        alerts[sym] = UP if diff > 0 else DOWN
    elif alerts.pop(sym, None):
        c.config(bg=BG)
    text = fmt(price, dec)
    p.config(text=text, font=("Consolas", 7 if len(text) > 9 else 9))   # 자릿수 많으면 축소
    c.config(text=f"{pct:+.2f}%", fg=UP if diff > 0 else DOWN if diff < 0 else DIM)


def blink(on=[False]):
    on[0] = not on[0]
    flash = next(iter(alerts.values()), None)          # 숨겨둬도 탭 버튼이 알려줌
    tab_btn.config(bg=flash if flash and on[0] else ACC,
                   fg="#e8ecf3" if flash and on[0] else "#0b0e14")
    for sym, color in list(alerts.items()):
        if sym in rows:
            c = rows[sym][2]
            c.config(bg=color if on[0] else BG, fg="#12151c" if on[0] else color)
    root.after(500, blink)


def schedule(sec):
    global timer
    if timer:
        root.after_cancel(timer)
    timer = root.after(int(sec * 1000), refresh)


def refresh():
    def work():
        keys = [split(x)[0] for x in visible()]
        groups = {"index": [], "stock": [], "yahoo": []}
        for k in keys:
            groups["index" if k in INDEX else "stock" if CODE.match(k) else "yahoo"].append(k)

        data, wait = {}, 0
        for kind in ("index", "stock"):
            syms = groups[kind]
            if syms:
                try:
                    got = naver(kind, [INDEX.get(s, s) for s in syms])
                    data.update({s: got[INDEX.get(s, s)] for s in syms if INDEX.get(s, s) in got})
                except Exception:
                    pass
        if groups["yahoo"]:
            try:
                data.update(yahoo(groups["yahoo"]))
            except urllib.error.HTTPError as e:
                wait = 60 if e.code == 429 else 0    # 요청 과다 -> 1분 쉬었다 재시도
            except Exception:
                pass

        root.after(0, status.config, {"text": "요청 과다 · 1분 대기" if wait else ""})
        for k in keys:
            root.after(0, paint, k, data.get(k))
        root.after(0, schedule, wait or cfg["refresh_sec"])

    Thread(target=work, daemon=True).start()


def settings(_=None):
    win = tk.Toplevel(root, bg=BG)
    win.title("설정")
    win.attributes("-topmost", True)
    win.attributes("-alpha", ALPHA)
    win.geometry(f"+{root.winfo_x() + 20}+{root.winfo_y() + 20}")
    tk.Label(win, text="목록 (한 줄에 하나 · 심볼=이름 · --- 는 구분선)", bg=BG, fg=DIM,
             font=("Malgun Gothic", 9)).pack(anchor="w", padx=12, pady=(12, 4))
    txt = tk.Text(win, width=30, height=12, bg=FIELD, fg=FG, insertbackground=FG,
                  relief="flat", font=("Consolas", 11))
    txt.insert("1.0", "\n".join(cfg["tickers"]))
    txt.pack(padx=12)
    tip = ("국내: 6자리 코드만 (069500, 0005A0) · 지수 KOSPI KOSDAQ\n"
           "환율 USDKRW=X · 코인 BTC-KRW · 미국 AAPL\n"
           "이름이 길면 069500=코덱스200 처럼 직접 지정")
    tk.Label(win, text=tip, bg=BG, fg=DIM, justify="left",
             font=("Malgun Gothic", 8)).pack(anchor="w", padx=12, pady=(4, 6))
    top = tk.BooleanVar(value=cfg["topmost"])
    tk.Checkbutton(win, text="항상 위에 표시", variable=top, bg=BG, fg=FG, selectcolor=FIELD,
                   activebackground=BG, activeforeground=FG, bd=0, highlightthickness=0,
                   font=("Malgun Gothic", 9)).pack(anchor="w", padx=8)
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
    tk.Button(bar, text="취소", command=win.destroy, bg=FIELD, fg=FG, relief="flat",
              font=("Malgun Gothic", 9), padx=10, cursor="hand2").pack(side="right", padx=6)


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


def work_area():
    """작업 표시줄을 뺀 화면 영역 (왼쪽, 위, 오른쪽, 아래)."""
    r = ctypes.wintypes.RECT()
    try:
        ctypes.windll.user32.SystemParametersInfoW(0x0030, 0, ctypes.byref(r), 0)
        return r.left, r.top, r.right, r.bottom
    except Exception:
        return 0, 0, root.winfo_screenwidth(), root.winfo_screenheight()


def place_panel():
    """저장된 위치(없으면 우하단)에 시세창을 놓고, 버튼을 그 위에 붙임."""
    root.update_idletasks()
    tab.update_idletasks()
    left, top, right, bottom = work_area()
    pw, ph = root.winfo_reqwidth(), root.winfo_reqheight()
    tw, th = tab.winfo_reqwidth(), tab.winfo_reqheight()
    if cfg.get("pos"):
        px, py = cfg["pos"]
    else:
        px, py = right - pw - MARGIN, bottom - ph - MARGIN
    px = min(max(px, left), max(left, right - pw))
    py = min(max(py, top), max(top, bottom - ph))
    root.geometry("+%d+%d" % (px, py))
    ty = py - th - 2
    if ty < top:                          # 위쪽에 붙였으면 버튼을 아래로
        ty = py + ph + 2
    tab.geometry("+%d+%d" % (px + pw - tw, ty))


def toggle(_=None):
    cfg["hidden"] = not cfg["hidden"]
    if cfg["hidden"]:
        root.withdraw()
    else:
        root.deiconify()
    place_panel()
    tab_btn.config(text="◀" if cfg["hidden"] else "▶")
    save(cfg)


tab_btn.bind("<Button-1>", toggle)

root.bind("<Enter>", lambda e: hover(True))
root.bind("<Leave>", lambda e: root.after(120, maybe_hide))

drag = {}


def move(e):
    left, top, right, bottom = work_area()
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


def drop(_=None):
    cfg["pos"] = [root.winfo_x(), root.winfo_y()]
    save(cfg)
    place_panel()                         # 버튼도 따라오게


root.bind("<Button-1>", lambda e: drag.update(x=e.x_root - root.winfo_x(), y=e.y_root - root.winfo_y()))
root.bind("<B1-Motion>", move)
root.bind("<ButtonRelease-1>", drop)
gear.bind("<Button-1>", lambda e: (settings(), "break")[1])
menu = tk.Menu(root, tearoff=0)
menu.add_command(label="새로고침", command=refresh)
menu.add_command(label="설정", command=settings)
menu.add_command(label="숨기기/보이기", command=toggle)
menu.add_command(label="위치 초기화", command=lambda: (cfg.update(pos=None), save(cfg), place_panel()))
menu.add_command(label="닫기", command=root.destroy)
root.bind("<Button-3>", lambda e: menu.tk_popup(e.x_root, e.y_root))
tab_btn.bind("<Button-3>", lambda e: menu.tk_popup(e.x_root, e.y_root))

build()
refresh()
blink()
if cfg["hidden"]:
    root.withdraw()
tab_btn.config(text="◀" if cfg["hidden"] else "▶")
place_panel()
root.after(2000, place_panel)             # 시세가 채워지면 크기가 바뀌므로 한 번 더
root.mainloop()
