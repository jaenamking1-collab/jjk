# 남에게 주는 '실시간시세.exe' 의 껍데기. 파이썬이 없는 PC 에서도 위젯을 돌린다.
# 하는 일: 최신 ticker.pyw 를 %LOCALAPPDATA%\ticker 에 받아 두고 그걸 그대로 실행한다.
# 그래서 exe 를 다시 받지 않아도 위젯 코드는 켤 때마다 최신이 된다(ticker.pyw 의 self_update 와 같은 이치).
# 빌드: .github/workflows/ticker-exe.yml (릴리스 태그 ticker)
import base64, json, os, runpy, sys, urllib.request

# ticker.pyw 가 쓰는 표준 모듈을 여기서 미리 불러 둔다 — PyInstaller 는 이 파일의 import 만 보고 묶는다.
import ctypes, ctypes.wintypes, datetime, html, re, shutil, subprocess, time, webbrowser  # noqa: F401
import tkinter, urllib.error, urllib.parse, threading  # noqa: F401

REPO = "jaenamking1-collab/jjk"
FILE = "tools/ticker.pyw"
MIRRORS = (("https://raw.githubusercontent.com/%s/main/%s" % (REPO, FILE), "raw"),
           ("https://api.github.com/repos/%s/contents/%s?ref=main" % (REPO, FILE), "api"),
           ("https://cdn.jsdelivr.net/gh/%s@main/%s" % (REPO, FILE), "jsdelivr"))
DEST = os.path.join(os.environ.get("LOCALAPPDATA") or os.path.expanduser("~"), "ticker")
ME = os.path.join(DEST, "ticker.pyw")


def fetch():
    for url, name in MIRRORS:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "ticker-widget"})
            body = urllib.request.urlopen(req, timeout=5).read()
            if name == "api":
                body = base64.b64decode(json.loads(body)["content"])
            if len(body) > 10000 and b"tkinter" in body:
                compile(body, ME, "exec")            # 깨진 코드는 받지 않는다
                return body, name
        except Exception:
            continue
    return None, None


os.makedirs(DEST, exist_ok=True)
new, src = fetch()
if new:
    try:
        old = open(ME, "rb").read() if os.path.exists(ME) else b""
        if new != old:
            with open(ME, "wb") as f:
                f.write(new)
        note = "최신(%s)" % src
    except Exception:
        note = "저장 실패"
elif os.path.exists(ME):
    note = "확인 실패(인터넷) — 있던 것으로 실행"
else:
    import tkinter.messagebox
    tkinter.Tk().withdraw()
    tkinter.messagebox.showerror("실시간 시세", "처음 실행에는 인터넷이 필요합니다.\n연결을 확인하고 다시 켜 주세요.")
    sys.exit(1)

os.environ["TICKER_LAUNCHER"] = note             # ticker.pyw 는 이걸 보고 자기 갱신을 건너뛴다
os.chdir(DEST)
try:
    runpy.run_path(ME, run_name="__main__")
except SystemExit:
    raise
except BaseException:
    # 창 없는 exe 라 오류가 그냥 사라진다. 받은 사람이 보내 줄 수 있게 파일로 남긴다.
    import traceback, tkinter.messagebox
    with open(os.path.join(DEST, "error.log"), "w", encoding="utf-8") as f:
        traceback.print_exc(file=f)
    tkinter.messagebox.showerror("실시간 시세", "오류로 꺼졌습니다.\n%s\\error.log 파일을 보내 주세요." % DEST)
    sys.exit(1)
