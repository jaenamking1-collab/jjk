"""다른 프로그램(노치 고양이 설정창)이 공용 종목 목록을 고치면, 켜져 있는 위젯이 다음 새로 고침 때 받아들이는지 본다.

가짜 tkinter 로 ticker.pyw 를 불러 **진짜 refresh()** 를 돌린다. 공용 파일은 임시 폴더의 것이고 git 은 부르지 않는다
(실제 ~/claude-memory 목록은 건드리지 않는다).

    python tools/test_ticker_adopt.py
"""
import json, os, sys, tempfile, types


class W:
    def __init__(s, *a, **k): pass
    def winfo_pointerxy(s): return (0, 0)
    def __getattr__(s, n):
        if n == "winfo_children": return lambda *a, **k: []
        if n.startswith("winfo_"): return lambda *a, **k: 0
        return lambda *a, **k: W()
    def after(s, ms, fn=None, *a): return None


tk = types.ModuleType("tkinter")
for n in ("Tk", "Toplevel", "Frame", "Label", "Canvas", "Menu", "StringVar", "IntVar", "BooleanVar", "Scale", "Entry", "Text", "Button", "Checkbutton"):
    setattr(tk, n, W)
tk.TclError = Exception
for sub in ("font", "ttk", "messagebox", "simpledialog", "colorchooser"):
    m = types.ModuleType("tkinter." + sub)
    m.__getattr__ = lambda n: W
    sys.modules["tkinter." + sub] = m
    setattr(tk, sub, m)
sys.modules["tkinter"] = tk

here = os.path.dirname(os.path.abspath(__file__))
src = open(os.path.join(here, "ticker.pyw"), encoding="utf-8").read()
src = src.replace("\nself_update()\n", "\n", 1)             # 시험에서는 자기 갱신을 끈다
d = tempfile.mkdtemp()
f = os.path.join(d, "ticker.pyw")
open(f, "w", encoding="utf-8").write(src)
os.environ["TICKER_CONFIG"] = os.path.join(d, "cfg.json")   # ⛔ 실제 목록을 덮어쓰지 않게(widget.md)
import urllib.request
def no_net(*a, **k): raise OSError("시험: 통신 없음")
urllib.request.urlopen = no_net
g = {"__name__": "__main__", "__file__": f}
exec(compile(src, f, "exec"), g)

shared = os.path.join(d, "shared.json")
start = list(g["cfg"]["tickers"])
synced, built = [], []
g["SHARED"] = shared
g["sync"] = lambda: synced.append(1)                         # git 은 부르지 않는다
real_build = g["build"]
g["build"] = lambda: (built.append(list(g["cfg"]["tickers"])), real_build())[1]

# 1) 공용 파일이 내 목록과 같으면 아무 일도 없다
json.dump({"tickers": start}, open(shared, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
assert g["adopt"]() is False and not built

# 2) 노치 고양이가 고쳤다 → 다음 refresh 에서 받아들이고 화면을 다시 짠다
new = ["KOSDAQ=코스닥"] + start[1:] + ["---", "XRP-KRW/KAIA-KRW=XRP당 카이아 @49,50,v41"]
json.dump({"tickers": new}, open(shared, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
g["refresh"]()
assert g["cfg"]["tickers"] == new and g["pushed"] == new, g["cfg"]["tickers"]
assert built == [new], built
assert json.load(open(os.environ["TICKER_CONFIG"], encoding="utf-8"))["tickers"] == new    # 그 PC 파일에도 남는다

# 3) 그 뒤 위젯 설정에서 다른 값만 저장해도 목록이 옛것으로 돌아가지 않는다
g["save"](g["cfg"])
assert json.load(open(shared, encoding="utf-8"))["tickers"] == new

# 4) 쓰다 만(깨진) 파일·빈 목록은 무시한다
open(shared, "w", encoding="utf-8").write("{깨진")
assert g["adopt"]() is False and g["cfg"]["tickers"] == new
json.dump({"tickers": []}, open(shared, "w", encoding="utf-8"))
assert g["adopt"]() is False and g["cfg"]["tickers"] == new

print("통과: 바뀐 공용 목록을 받아들임(%d줄 → %d줄), 화면 다시 짬 %d번, 깨진 파일·빈 목록 무시" % (len(start), len(new), len(built)))
