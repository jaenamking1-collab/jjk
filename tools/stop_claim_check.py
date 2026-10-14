"""Stop 훅: 이번 답에서 '됐습니다/고쳤습니다' 류로 끝냈는데 도구를 하나도 안 썼으면(확인 없이 장담) 한 번 되돌린다.
사용자: '하도 속아서 의심덩어리가 됐다'(2026-09-29). hookify 는 대화 전체만 봐서 이번 답만 검사할 수 없어 직접 만든다(2026-10-14)."""
import json, re, sys

CLAIM = re.compile(r'(됐습니다|되었습니다|고쳤습니다|해결했습니다|완료했습니다|끝냈습니다|반영했습니다|배포했습니다|수정했습니다|넣었습니다)')

QUOTED = re.compile(r'"[^"\n]*"|“[^”]*”|‘[^’]*’|「[^」]*」|`[^`]*`|```.*?```', re.S)


def turn_tool_count(path):
    """마지막 '사람이 친' 메시지 뒤로 assistant 가 부른 도구 수."""
    lines = open(path, encoding='utf-8', errors='replace').read().splitlines()
    n = 0
    for ln in reversed(lines):
        try:
            o = json.loads(ln)
        except ValueError:
            continue
        msg = o.get('message') or {}
        c = msg.get('content')
        if o.get('type') == 'assistant' and isinstance(c, list):
            n += sum(1 for b in c if isinstance(b, dict) and b.get('type') == 'tool_use')
        elif o.get('type') == 'user' and not o.get('isMeta'):
            human = isinstance(c, str) or (isinstance(c, list) and not any(isinstance(b, dict) and b.get('type') == 'tool_result' for b in c))
            if human:
                break
    return n


def judge(inp):
    if inp.get('stop_hook_active'):
        return None  # 이미 한 번 되돌렸다 — 무한 반복 금지
    text = inp.get('last_assistant_message') or ''
    # 따옴표·코드 안의 말은 장담이 아니다(기능 이름 '"됐습니다" 검사'에 걸린 오작동, 2026-10-14)
    text = QUOTED.sub('', text)
    if not CLAIM.search(text):
        return None
    p = inp.get('transcript_path')
    if not p or turn_tool_count(p) > 0:
        return None
    return ('확인 없이 "완료"를 말했습니다(이번 답에서 도구를 한 번도 안 씀). '
            '실제로 확인하고(명령·응답값·화면) 그 증거를 같이 말하세요. 확인할 수 없는 일이면 "확인 못 함"이라고 고쳐 말하세요.')


if __name__ == '__main__':
    try:
        why = judge(json.loads(sys.stdin.buffer.read().decode('utf-8') or '{}'))
    except Exception:
        why = None
    if why:
        sys.stdout.buffer.write(json.dumps({'decision': 'block', 'reason': why}, ensure_ascii=False).encode('utf-8'))
