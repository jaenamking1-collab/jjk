"""코덱스 사용량 감시: 5시간 한도(또는 주간 한도)가 90% 쓰이면 코덱스를 끊고 알린다.

코덱스는 스스로 남은 사용량을 지키지 못한다(2026-11-26: "10% 남기라" 해 놓고 100%까지 씀).
그래서 밖에서 본다. 코덱스는 작업 중 ~/.codex/sessions/**/rollout-*.jsonl 에
rate_limits.primary.used_percent 를 계속 적는다. 그 숫자를 5초마다 읽는다.

- 끊는 방법: codex.exe 를 강제 종료한다. 앱은 다음 메시지 때 다시 띄운다.
- 한 한도 구간(resets_at)마다 한 번만 끊는다. 끊긴 뒤 사용자가 일부러 이어 하면 막지 않는다.
- 켜질 때 이미 넘은 구간은 끊지 않는다(놀고 있는 앱을 괜히 죽이지 않게).
실행: 코덱스감시_실행.bat  /  자체 점검: python codex_guard.pyw --test
"""
import ctypes, glob, json, os, subprocess, sys, threading, time

LIMIT = 90.0          # 쓴 비율이 이 이상이면 끊는다 = 10% 남음
POLL = 5
SESS = os.path.join(os.path.expanduser("~"), ".codex", "sessions")


def read_limits(path, tail=300_000):
    """파일 끝부분에서 마지막 codex rate_limits 를 꺼낸다 → [(used, resets_at), ...]"""
    try:
        with open(path, "rb") as f:
            f.seek(max(0, os.path.getsize(path) - tail))
            lines = f.read().decode("utf-8", "ignore").splitlines()
    except OSError:
        return []
    for line in reversed(lines):
        if '"used_percent"' not in line:
            continue
        try:
            rl = json.loads(line)["payload"]["rate_limits"]
        except (ValueError, KeyError, TypeError):
            continue
        if rl.get("limit_id") != "codex":
            continue
        return [(w["used_percent"], w["resets_at"]) for w in (rl.get("primary"), rl.get("secondary")) if w]
    return []


def over(windows, now):
    """아직 안 끝난 구간 중 한도를 넘은 것의 resets_at 목록"""
    return [r for u, r in windows if u >= LIMIT and r > now]


def recent_files(minutes=15):
    cut = time.time() - minutes * 60
    return [p for p in glob.glob(os.path.join(SESS, "**", "rollout-*.jsonl"), recursive=True)
            if os.path.getmtime(p) > cut]


def scan(now):
    hits = set()
    for p in recent_files():
        hits.update(over(read_limits(p), now))
    return hits


def popup(msg):
    threading.Thread(target=ctypes.windll.user32.MessageBoxW,
                     args=(0, msg, "코덱스 사용량 감시", 0x40 | 0x40000), daemon=True).start()


def main():
    # 한 번만 돌게
    ctypes.windll.kernel32.CreateMutexW(None, False, "jjk_codex_guard")
    if ctypes.windll.kernel32.GetLastError() == 183:
        return
    done = scan(time.time())          # 켜질 때 이미 넘은 구간은 건너뛴다
    while True:
        time.sleep(POLL)
        now = time.time()
        new = scan(now) - done
        if not new:
            continue
        done |= new
        subprocess.run(["taskkill", "/F", "/IM", "codex.exe"], capture_output=True,
                       creationflags=0x08000000)
        at = time.strftime("%H:%M", time.localtime(min(new)))
        popup(f"코덱스 사용량이 {LIMIT:.0f}%를 넘어 작업을 끊었습니다.\n"
              f"{at}에 한도가 다시 채워집니다.\n\n"
              f"고치다 만 파일이 있을 수 있으니 이어 하기 전에 확인하세요.\n"
              f"이번 구간에서 일부러 이어 하시면 다시 끊지 않습니다.")


def _test():
    import tempfile
    now = 1_000
    line = lambda lid, u: json.dumps({"payload": {"rate_limits": {"limit_id": lid,
        "primary": {"used_percent": u, "resets_at": 2_000}, "secondary": {"used_percent": 10, "resets_at": 9_000}}}})
    with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        f.write(line("codex", 95) + "\n" + line("premium", 99) + "\n")
    w = read_limits(f.name)
    assert w == [(95, 2_000), (10, 9_000)], w          # premium 줄은 건너뛰고 codex 마지막 값
    assert over(w, now) == [2_000]
    assert over(w, 3_000) == []                        # 이미 재설정된 구간은 무시
    assert over([(89.9, 2_000)], now) == []
    os.unlink(f.name)
    print("ok")


if __name__ == "__main__":
    _test() if "--test" in sys.argv else main()
