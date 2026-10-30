# 남에게 주는 'RealtimeTicker.exe' 의 진입점. 파이썬이 없는 PC 에서도 위젯을 돌린다.
# 위젯 코드(ticker_exe.py = ticker.pyw 에서 자기 갱신을 뺀 것, make_exe.py 가 만든다)를 품고 돈다.
# 예전엔 켤 때마다 최신 ticker.pyw 를 받아 실행했는데, 그 모양을 백신이 셸로더로 봤다
# (2026-10-30 VirusTotal 8/71). 이제 코드는 받지 않고, 새 exe 가 나오면 위젯이 '새 버전 있음' 만 띄운다.
# 빌드: .github/workflows/ticker-exe.yml (릴리스 태그 ticker)
import os, sys

DEST = os.path.join(os.environ.get("LOCALAPPDATA") or os.path.expanduser("~"), "ticker")
os.makedirs(DEST, exist_ok=True)
os.chdir(DEST)
# 설정은 늘 같은 자리에 — exe 는 실행할 때마다 임시 폴더에 풀리므로 그 옆에 두면 사라진다
os.environ.setdefault("TICKER_CONFIG", os.path.join(DEST, "ticker_config.json"))
try:
    import ticker_exe  # noqa: F401  (불러오는 것이 곧 위젯 실행)
except SystemExit:
    raise
except BaseException:
    # 창 없는 exe 라 오류가 그냥 사라진다. 받은 사람이 보내 줄 수 있게 파일로 남긴다.
    import traceback, tkinter.messagebox
    with open(os.path.join(DEST, "error.log"), "w", encoding="utf-8") as f:
        traceback.print_exc(file=f)
    tkinter.messagebox.showerror("실시간 시세", "오류로 꺼졌습니다.\n%s\\error.log 파일을 보내 주세요." % DEST)
    sys.exit(1)
