# -*- coding: utf-8 -*-
"""jjk 공개 저장소 경고 훅 (2026-11-25 사용자 요청: "꼭 훅을 걸어서 얘기해줘").

jjk 는 public 이고 Stop 훅이 매 턴 자동 커밋·푸시한다. 개인·보안 자료가 올라가려 하면 멈추고 알린다.

쓰는 법
  python public-guard.py staged    # Stop·SessionStart 훅: git add -A 뒤 스테이징된 것 검사.
                                   #   걸리면 스테이징을 풀고(git reset) 알림 JSON 출력, 종료코드 1
  python public-guard.py pretool   # PreToolUse(Bash/PowerShell): git commit/push 명령이면
                                   #   작업트리 변경+안 올라간 커밋 검사, 걸리면 종료코드 2 로 막음
통과시키기: 사용자가 괜찮다고 한 뒤에만 명령 앞에 PUBLIC_GUARD_OK=1 을 붙인다
  (PowerShell 은 $env:PUBLIC_GUARD_OK=1; git commit ...).
"""
import json, os, re, subprocess, sys

SELF = '.claude/hooks/public-guard.py'

# 경로에 들어 있으면 걸리는 것(학교·개인 자료 폴더)
PATH_WORDS = ['개인적인거', '정보부장', '금융/', '/backup/', 'backup/', '세특', '생기부', '학생명단', '썰/']
# 올라가면 안 되는 문서 확장자(학교 공문·금융 내역)
PATH_EXT = re.compile(r'\.(hwp|hwpx|pdf|docx?|xlsx?|xlsm|csv|pptx?)$', re.I)

CONTENT = [
    ('주민등록번호', re.compile(r'(?<!\d)\d{6}-[1-4]\d{6}(?!\d)')),
    ('휴대폰 번호', re.compile(r'(?<!\d)01[016789]-?\d{3,4}-?\d{4}(?!\d)')),
    # 2~5토막, 숫자 합계 10~16자리(날짜·버전은 8자리 이하라 안 걸림), 휴대폰 번호는 위에서 따로
    ('계좌번호로 보이는 숫자', re.compile(r'(?<![\d.-])(?!01[016789]-)\d{2,6}(?:-\d{1,7}){1,4}(?![\d.-])')),
    ('GitHub 토큰', re.compile(r'\b(ghp|gho|ghs|ghu)_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}')),
    ('Anthropic/OpenAI 키', re.compile(r'\bsk-(ant-)?[A-Za-z0-9_-]{30,}')),
    ('구글 API 키', re.compile(r'\bAIza[0-9A-Za-z_-]{35}')),
    ('AWS 키', re.compile(r'\bAKIA[0-9A-Z]{16}\b')),
    ('슬랙 토큰', re.compile(r'\bxox[abpr]-[A-Za-z0-9-]{10,}')),
    ('개인 키 파일', re.compile(r'-----BEGIN [A-Z ]*PRIVATE KEY-----')),
    ('비밀번호 값', re.compile(r'(password|passwd|비밀번호|비번|APP_TOKEN)\s*[:=]\s*["\']?[^\s"\'•*]{4,}', re.I)),
]


def git(*args):
    r = subprocess.run(['git', *args], capture_output=True)
    return r.stdout.decode('utf-8', 'replace'), r.returncode


def scan_diff(text):
    """'+' 줄만 본다. 반환: [(파일, 종류, 미리보기)]"""
    hits, cur = [], None
    for line in text.splitlines():
        if line.startswith('+++ '):
            p = line[4:]
            cur = p[2:] if p.startswith('b/') else p
            continue
        if not line.startswith('+') or cur in (None, '/dev/null', SELF):
            continue
        body = line[1:]
        for name, rx in CONTENT:
            if name.startswith('계좌'):
                if '사업자' in body:
                    continue  # 운용사 공지의 사업자등록번호(공개 정보)
                m = next((x for x in rx.finditer(body)
                          if 10 <= sum(c.isdigit() for c in x.group(0)) <= 16), None)
            else:
                m = rx.search(body)
            if m:
                s = m.group(0)
                hits.append((cur, name, s[:4] + '…' if len(s) > 6 else s))
    return hits


def scan_paths(paths):
    hits = []
    for p in paths:
        if not p or p == SELF:
            continue
        for w in PATH_WORDS:
            if w in p:
                hits.append((p, '개인·학교 자료 폴더(' + w.strip('/') + ')', ''))
                break
        else:
            if PATH_EXT.search(p):
                hits.append((p, '문서 파일(' + p.rsplit('.', 1)[-1] + ')', ''))
    return hits


def untracked_diff():
    out, _ = git('ls-files', '--others', '--exclude-standard', '-z')
    files = [f for f in out.split('\0') if f]
    chunks = []
    for f in files:
        try:
            with open(f, 'rb') as fh:
                data = fh.read(2_000_000)
        except OSError:
            continue
        if b'\0' in data[:8000]:
            continue  # 바이너리
        txt = data.decode('utf-8', 'replace')
        chunks.append('+++ b/' + f + '\n' + '\n'.join('+' + l for l in txt.splitlines()))
    return files, '\n'.join(chunks)


def report(hits):
    seen, lines = set(), []
    for f, k, s in hits:
        key = (f, k)
        if key in seen:
            continue
        seen.add(key)
        lines.append('· ' + f + ' — ' + k + (' (' + s + ')' if s else ''))
    more = '' if len(lines) <= 8 else '\n· …외 %d건' % (len(lines) - 8)
    return '\n'.join(lines[:8]) + more


def out(s, stream):
    stream.buffer.write(s.encode('utf-8'))
    stream.flush()


def mode_staged():
    names, _ = git('diff', '--cached', '--name-only', '-z')
    diff, _ = git('diff', '--cached', '-U0', '--no-color')
    hits = scan_paths([n for n in names.split('\0') if n]) + scan_diff(diff)
    if not hits:
        return 0
    git('reset', '-q')
    msg = ('🔒 jjk는 공개 저장소라 자동 커밋·푸시를 멈췄습니다. 개인·보안 자료로 보이는 것이 있습니다:\n'
           + report(hits) +
           '\n괜찮은 내용이면 "올려도 돼", 아니면 "빼"라고 말씀해 주세요.')
    out(json.dumps({'systemMessage': msg}, ensure_ascii=True), sys.stdout)
    return 1


def mode_pretool():
    try:
        data = json.loads(sys.stdin.buffer.read().decode('utf-8', 'replace') or '{}')
    except ValueError:
        return 0
    cmd = (data.get('tool_input') or {}).get('command') or ''
    if not re.search(r'\bgit\b[^\n|;&]*\b(commit|push)\b', cmd):
        return 0
    if 'PUBLIC_GUARD_OK=1' in cmd:
        return 0
    # 이 저장소에서 도는 명령만: cd·git -C 로 다른 저장소(claude-memory 등)에 가면 건너뜀
    here = os.path.normcase(os.path.realpath(os.getcwd()))
    for t in re.findall(r'(?:\bcd|\bgit\s+-C)\s+["\']?([^\s"\';&|]+)', cmd):
        t = os.path.expanduser(t.replace('$HOME', '~'))
        if os.path.normcase(os.path.realpath(t)) != here:
            return 0
    hits = []
    if re.search(r'\bcommit\b', cmd):
        names, _ = git('diff', 'HEAD', '--name-only', '-z')
        diff, _ = git('diff', 'HEAD', '-U0', '--no-color')
        ufiles, udiff = untracked_diff()
        hits += scan_paths([n for n in names.split('\0') if n] + ufiles) + scan_diff(diff) + scan_diff(udiff)
    up, rc = git('rev-parse', '--abbrev-ref', '@{u}')
    base = up.strip() if rc == 0 else 'origin/main'
    names, _ = git('diff', base + '..HEAD', '--name-only', '-z')
    diff, _ = git('diff', base + '..HEAD', '-U0', '--no-color')
    hits += scan_paths([n for n in names.split('\0') if n]) + scan_diff(diff)
    if not hits:
        return 0
    out('🔒 jjk는 공개 저장소입니다. 개인·보안 자료로 보이는 것이 있어 커밋·푸시를 막았습니다:\n'
        + report(hits) +
        '\n사용자에게 무엇이 걸렸는지 그대로 알리고, 빼거나(.gitignore·.git/info/exclude·값 지우기) '
        '사용자가 "올려도 돼"라고 한 뒤에만 명령 앞에 PUBLIC_GUARD_OK=1 을 붙여 다시 실행하세요.\n', sys.stderr)
    return 2


if __name__ == '__main__':
    if os.environ.get('CLAUDE_PROJECT_DIR'):
        os.chdir(os.environ['CLAUDE_PROJECT_DIR'])
    if os.path.basename(git('rev-parse', '--show-toplevel')[0].strip()) != 'jjk':
        sys.exit(0)  # jjk 밖이면 아무것도 안 함
    mode = sys.argv[1] if len(sys.argv) > 1 else ''
    if mode == 'staged':
        sys.exit(mode_staged())
    if mode == 'pretool':
        sys.exit(mode_pretool())
    sys.exit(0)
