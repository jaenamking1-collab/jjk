# exe 빌드 전에 돈다(.github/workflows/ticker-exe.yml): ticker.pyw 에서 [exe-cut]~[/exe-cut]
# (인터넷에서 코드 받기·자기 교체·git 호출)을 빼 tools/ticker_exe.py 로 쓴다.
# exe 는 이 파일을 품고 돌며, 새 버전은 위젯의 exe_update_check 가 알려만 준다(2026-10-30 VirusTotal 8/71).
import os, re

HERE = os.path.dirname(os.path.abspath(__file__))
src = open(os.path.join(HERE, "ticker.pyw"), encoding="utf-8").read()
cut = re.compile(r"# \[exe-cut\].*?# \[/exe-cut\]\n", re.S)
assert len(cut.findall(src)) == 1, "exe-cut 표시가 하나여야 한다"
out = cut.sub('update_note = ["exe"]\n\n\ndef git(*_a, **_k):\n    return 1\n', src)
out = out.replace(" shutil, subprocess, sys,", " shutil, sys,", 1)
for bad in ("subprocess", "execv", "fetch_code", "raw.githubusercontent.com/%s/main/%s\" % (REPO, FILE)"):
    assert bad not in out, bad
compile(out, "ticker_exe.py", "exec")
open(os.path.join(HERE, "ticker_exe.py"), "w", encoding="utf-8").write(out)
print("ticker_exe.py", len(out), "bytes")
