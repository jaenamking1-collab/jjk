"""위젯이 '어느 연결로 시세를 받나'를 윈도우 없이 확인한다.

가짜 tkinter 와 가짜 네트워크를 끼우고 ticker.pyw 의 **진짜 refresh()** 를 돌린다.
망을 하나씩 막아 가며, 골라 쓰는 경로와 화면에 밝히는 출처가 맞는지 본다.

    빗썸 직접 > 업비트 직접 > 코인게코(빗썸 중계) > 야후 x 프리미엄 환산

⚠️ 화면에 뜨는 출처 문구가 실제 데이터 경로와 다르면 그게 버그다. 좋아 보이게도,
   나빠 보이게도 쓰면 안 된다 — 둘 다 틀려 본 적이 있다(WORKLOG 170 부근).

    python3 tools/test_ticker_sources.py
"""
import sys, types, time, json, datetime


class W:
    def __init__(s,*a,**k): pass
    def winfo_pointerxy(s): return (0,0)
    def __getattr__(s,n):
        if n == "winfo_children": return lambda *a,**k: []
        if n.startswith("winfo_"): return lambda *a,**k: 0
        return lambda *a,**k: W()
    def after(s,ms,fn=None,*a):
        if fn is not None and ms == 0:
            try: fn(*a)
            except Exception: pass
        return None
tk=types.ModuleType("tkinter")
for n in ("Tk","Toplevel","Frame","Label","Canvas","Menu","StringVar","IntVar","BooleanVar","Scale","Entry","Text","Button","Checkbutton"):
    setattr(tk,n,W)
tk.TclError=Exception
for sub in ("font","ttk","messagebox","simpledialog","colorchooser"):
    m=types.ModuleType("tkinter."+sub); m.__getattr__=lambda n: W
    for nn in ("Font","families","Style","showinfo","showerror","askstring","askcolor","Combobox"):
        setattr(m,nn,W)
    sys.modules["tkinter."+sub]=m; setattr(tk,sub,m)
sys.modules["tkinter"]=tk

MODE={"bithumb":True,"upbit":True,"yahoo":True,"gecko":True}
def fake_get(url, *a, **k):
    if "bithumb.com" in url:
        if not MODE["bithumb"]: raise OSError("blocked")
        sym=url.rstrip("/").split("/")[-1].split("_")[0]
        px={"XRP":1990.0,"KAIA":42.50}[sym]
        return {"status":"0000","data":{"closing_price":str(px),"prev_closing_price":str(px*0.99),
                "date":str(int(time.time()*1000))}}
    if "upbit.com" in url:
        if not MODE["upbit"]: raise OSError("blocked")
        raise OSError("no data")
    if "coingecko.com" in url:
        if not MODE["gecko"]: raise OSError("blocked")
        if "simple/price" in url:
            return {"ripple":{"krw":1985.0},"kaia":{"krw":42.10}}
        if "market_chart" in url:
            h0=int(time.time()//3600)*3600000
            v = 1985.0 if "ripple" in url else 42.10
            return {"prices":[[h0-i*3600000, v*(1+0.001*i)] for i in range(24*31)]}
        return {"tickers":[{"base":"XRP","target":"KRW","last":1985.0,"is_stale":False},
                           {"base":"KAIA","target":"KRW","last":42.10,"is_stale":False}]}
    if "yahoo" in url or "query" in url:
        if not MODE["yahoo"]: raise OSError("blocked")
        import urllib.parse as _u
        known={"XRP-USD":1.40,"KAIA-USD":0.0300,"USDKRW=X":1400.0}
        out={}
        for s in _u.unquote(url.split("symbols=")[-1].split("&")[0]).split(","):
            if s not in known: continue            # 야후에 없는 심볼은 빠져서 온다(XRP-KRW 등)
            p=known[s]
            out[s]={"meta":{"regularMarketPrice":p,"chartPreviousClose":p*0.99,
                            "currency":"USD" if s.endswith("-USD") else "KRW"}}
        return {"spark":{"result":[{"symbol":k,"response":[v]} for k,v in out.items()]}}
    raise OSError("blocked")

src=open("tools/ticker.pyw",encoding="utf-8").read()
src=src.replace("\nself_update()\n", "\n", 1)          # 테스트에서는 자기 갱신을 끈다
import os, tempfile
d=tempfile.mkdtemp(); f=os.path.join(d,"ticker.pyw"); open(f,"w",encoding="utf-8").write(src)
g={"__name__":"__main__","__file__":f}
exec(compile(src,f,"exec"),g)
g["get"]=fake_get
g["cfg"]["coin_base"]={}
# 종목: XRP, KAIA, 그리고 교환비율 줄
g["cfg"]["tickers"]=["XRP-KRW=XRP","KAIA-KRW=카이아","XRP-KRW/KAIA-KRW=XRP당 카이아 @47,48,49,50"]
g["cfg"]["collapsed"]=[False]*8
g["build"](); 

def run(label, stale=True, **mode):
    MODE.update(mode)
    g["coin_at"][0]=0; g["gecko_at"][0]=0; g["gecko_key"][0]=""
    if stale and not (MODE["bithumb"] or MODE["upbit"]):
        g["coin_ok"][0]=time.time()-60          # 거래소가 한참 막혀 있는 상태로 둔다
    if MODE["yahoo"]:
        g["coin_ext"][0]=True; g["ext_off"][0]=0    # 야후가 살아났으면 다시 물어보게 둔다
    for _ in range(3):              # 달러 페어 학습에 한 사이클이 걸린다. 실제로도 5초 뒤다.
        g["last"].clear(); g["coin_at"][0]=0; g["gecko_at"][0]=0
        if stale and not (MODE["bithumb"] or MODE["upbit"]):
            g["coin_ok"][0]=time.time()-60
        g["refresh"]()
        for _ in range(60):
            time.sleep(0.05)
            if g["last"]: break
    px={k:(round(v[0],2), round(v[2],2)) for k,v in sorted(g["last"].items())} if "last" in g else {}
    print("%-26s 출처: %-34s %s" % (label, g["coin_src"][0] or "(빗썸 직접)", px))

print("종목:", g["visible"](), "\n")
run("① 집: 빗썸 직접")
run("② 직장: 빗썸/업비트 막힘", bithumb=False, upbit=False)
run("③ 빗썸+야후 둘 다 막힘", bithumb=False, upbit=False, yahoo=False)
run("④ 전부 막힘", bithumb=False, upbit=False, yahoo=False, gecko=False)
g["gecko_px"].clear(); g["gecko_ok"][0]=0
run("⑤ 게코도 막힘, 야후만 (환산)", bithumb=False, upbit=False, yahoo=True, gecko=False)
g["gecko_px"].clear(); g["gecko_ok"][0]=0
run("⑥ 전부 막힘 (값 굳음)", bithumb=False, upbit=False, yahoo=False, gecko=False)
run("⑦ 집으로 복귀", bithumb=True, upbit=True, yahoo=True, gecko=True)
