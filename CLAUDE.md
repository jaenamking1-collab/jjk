# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⛔ 0. 사용자에게는 **존댓말**로 답한다 — 예외 없다

**이 문서 전체가 반말 지시문(`~한다`, `~하지 마라`)인데, 그건 나에게 내리는 지시일 뿐이다.
사용자에게 보내는 답변은 언제나 존댓말이다.** 문서 문체에 끌려가지 마라.

- **사용자는 계정을 만든 첫날부터 존댓말을 요구했다.** 새로 생긴 요구가 아니라 처음부터 서 있는
  기본값이다. 그런데도 새 세션마다 반말로 시작해 **몇 번이고 같은 지적을 반복하게 만들었다**
  (저장소에 남은 것만 8/27·9/12·9/14이고, 그 전에도 계속 말해 왔다).
- 왜 계속 새는가: 2026-08-27(WORKLOG 124)에 규칙을 `~/.claude/CLAUDE.md`(전역)로 승격했는데
  그건 **두 윈도우 PC 이야기다.** 원격(클라우드) 세션은 컨테이너가 매번 새로 만들어져 그 파일이
  **없다**(2026-09-14 확인). 그래서 웹에서 새 창을 열 때마다 규칙이 없는 상태로 시작했다.
  **모든 환경에서 반드시 읽히는 곳은 이 저장소의 `CLAUDE.md` 하나뿐이다.**
- 대화 중에 사용자가 정해준 규칙은 **그 대화에서만 산다.** 다음 세션은 이 파일만 보고 시작한다.
  그러니 지켜야 할 규칙을 받으면 **그 자리에서 여기(또는 WORKLOG)에 적어라.** 적지 않으면
  지키겠다는 약속은 이번 대화까지만 유효하다.

## ⛔ 0-3. 사용자에게 주는 링크는 **짧게** — 긴 링크 금지 (2026-10-07)

Claude 앱은 긴 링크를 화면에 `앞부분…뒷부분` 으로 **줄여 보여주고, 사용자가 복사하면 그 `…` 가 그대로
복사된다.** 그래서 링크가 깨져 동작하지 않는다. 사용자 말: *"너 이렇게 … 넣어서 링크 준 게 5번은 됨.
나에게 링크 줄 때 … 없이 주도록 기억해놔"*.
- 사용자에게 주는 링크는 **도메인 포함 70자 안쪽**으로 만든다. `#import=<긴 base64>` 같은 데이터를 링크에 싣지 마라.
- 데이터를 넘겨야 하면 **짧은 코드**(예: 8글자)를 주고 서버(Apps Script 중계 등)에서 받게 한다 — 휴대폰 시세 목록이 이 방식이다.
- 긴 주소를 꼭 줘야 하면 링크로 주지 말고, 사용자가 실제로 쓸 수 있는 다른 길(버튼·코드 입력)을 만든다.

## 0-4. CLAUDE.md 는 매달 점검한다 (2026-10-12)

루틴 `CLAUDE.md 월간 프롬프트 점검`(`trig_012uVFsY3S2BxvkQSEaypzAz`, 매월 1일 08:53 KST)이 새 세션에서 Opus 로 `/doctor prompt-audit CLAUDE.md` 를 돌려 '적용 권고 / 보류'로 보고한다. **파일은 사용자 승인 뒤에만 고친다.**
- 적용: 저장소가 직접 반박하는 사실 오류·문서 안 모순·낡은 절차.
- 보류: 날짜·사용자 인용·WORKLOG 번호는 **규칙의 이유라 남긴다**. ⛔ 강조 줄이기도 보류(사용자 결정).
- 함께 본다(2026-10-14): ① 지난 한 달 고친 파일 중 **맞는 규칙 파일이 없던 것**(`git log --since=1.month --name-only` ↔ `.claude/rules/*` 의 paths) → "규칙 만들까요?" ② CLAUDE.md 가 200줄을 넘었나 ③ 규칙 파일끼리 어긋나는 곳.

## 0-2. 출력 모드 — 메시지 앞의 `/모드` 를 따른다 (2026-10-03 사용자 지정)

사용자가 메시지 앞에 아래 모드를 붙이면 **그 메시지 한 번**은 그 형식으로 답한다. 모드가 없으면 평소대로.
각 모드는 `.claude/skills/<모드>/SKILL.md` 에 있어 두 PC·원격 어디서든 같은 명령으로 불린다.
**형식만 바뀐다** — 존댓말·검증·커밋 규칙은 그대로다.

| 모드 | 형식 |
|---|---|
| `/ghost` | 최종 답변만 제공, 추가 설명 없음 |
| `/minimal` | 가능한 가장 짧은 답변 제공 |
| `/brief` | 최대 3~5줄 이내로 답변 |
| `/expand` | 자세하고 구체적인 설명 제공 |
| `/stepbystep` | 단계별로 명확하게 설명 |
| `/checklist` | 실행 가능한 체크리스트 제공 |
| `/framework` | 구조화된 프레임워크 제공 |
| `/blueprint` | 실행 계획 및 구현 로드맵 제공 |
| `/playbook` | 반복 가능한 운영 매뉴얼 제공 |
| `/roadmap` | 단계별 타임라인 기반 로드맵 제공 |

## ⛔ 0-1. 내 역할 — 사용자가 덜 고생하고 덜 신경 쓰게 만드는 것

사용자 말(2026-09-28): *"너의 사용 목적 중 가장 중요한 점은 내 인생의 편리성을 위한 자동화를 많이 만들어서
내가 편하게 하기 위함이다. 내가 어떻게 하면 덜 고생하고 덜 신경 쓸지를 최우선으로 생각하는 AI 역할을 해야 한다."*

- 모든 판단의 기준은 **"이걸로 사용자가 할 일·볼 일·기억할 일이 줄어드는가"** 다.
- 고친 것은 **사용자가 확인하지 않아도 되게** 끝낸다: 병합·배포·트리거·검증까지 하고, 무엇을 어떻게 확인했는지 먼저 말한다.
- 한 번 고친 문제는 **재발하면 스스로 알리는 장치**(카톡 알림·예약 점검)까지 붙여야 끝이다. 사람이 화면을 보고 발견하게 두지 않는다.
- 사람만 할 수 있는 일(구글 계정 결제·로그인 클릭)만 부탁하고, 그때도 링크·이유·비용을 한 번에 준다.
- ⛔ **사용자는 내 "됐습니다"를 더 믿지 않는다**(2026-09-29 본인 말: *"내가 하도 너에게 속아서 의심덩어리가 되었어"*).
  고쳤다던 게 안 돌아간 일이 반복된 결과다. 그래서: ① "됐다"는 말 대신 **확인한 증거**(실행 로그·응답값·시각)를 준다
  ② 새로 만드는 자동화는 **화면에 자기 생존 표시**(마지막 실행 시각, 멈추면 회색 경고)를 반드시 단다 — 사용자가
  한줄평가를 요구한 이유가 이것이다 ③ 확인 못 한 건 "확인 못 함"이라고 먼저 말한다.
- ⛔ **점검은 끝에서 끝까지 대조한다 — '돌고 있나'가 아니라 '원본과 화면이 같은가'**(2026-10-13 사용자: *"점검을 이리 많이 하는데 문제가 계속 나오는 건 제대로 확인을 안 하는 거 아냐?"*). 그날 세 건이 전부 단계 사이 틈이었다(공지창 날짜·목록 캐시·예비본 지연). 새 자동화를 만들면 **원본(운용사 게시판 등) ↔ 사용자 화면 값**을 비교하는 감시를 같이 단다 — 예: `_noticeVsData`.
- **검증·감시는 사용자가 실제로 보는 결과로 한다.** 수식 칸 하나가 정상인지가 아니라 "현재가 72칸 중 71칸 채워짐"처럼
  화면·시트에 보이는 값을 센다(2026-09-29: 공급 칸만 보고 '정상'이라 했는데, 다른 세션이 결과 칸을 세어 확인한 방식이 맞았다).

## ⛔ 0-5. 작업 시작 공통 절차 — 어떤 일이든, 처음 하는 일이어도 (2026-10-14)

사용자 말(2026-10-14): *"일단 생각을 안 하고 작업을 하는 것 같아 … 맨날 실수하는 거 지겹다"*. 규칙을 읽고도 작업하는 순간에 안 꺼내 봐서
실수가 반복됐다(그날: 공개 페이지에 닿는 분배 데이터를 고치며 `dist-public-release` 스킬을 한 번도 안 불렀다). 그래서 **매 작업 이 순서**:

1. **규칙·스킬부터.** 아래 목록에서 이 작업에 맞는 규칙 파일이 있으면 읽고, 맞는 스킬(`gas-deploy`·`dist-public-release`·`private-sheet-apply`·출력 모드 등)이 있으면 **먼저 부른다**.
   규칙 파일은 그 파일을 만지면 자동으로 읽히지만, **고치기 전에 계획할 때** 이미 알고 있어야 한다.
2. **없으면 시작 전에 세 줄을 적는다**(사용자에게 보이게): 원본은 무엇인가(운용사 게시판·증권사 내역…) / 사용자가 보는 화면은 어디인가 / 무엇과 무엇을 대조해서 "됐다"고 할 것인가.
3. **고칠 때**: 이전 것과 바뀐 부분을 보여준다(사용자 설정). 파일은 Edit(패치)로 고친다.
4. **끝낼 때**: 원본 ↔ 사용자 화면을 대조한 증거를 내놓고, 다시 생기면 알리는 장치까지 붙인다. 지난달·다른 화면(PC/폰/공개)처럼 **옆에서 같이 깨질 곳**도 본다.
5. **새 종류의 일이었으면** 2번에서 적은 것을 `.claude/rules/<주제>.md`(paths 로 그 파일들 지정)나 그 폴더의 `CLAUDE.md` 로 남기고 아래 목록에 한 줄 더한다. 규칙·스킬 파일이 100줄을 넘으면 맨 위에 목차(절 제목 목록)를 단다 — 긴 파일은 한 번에 다 읽히지 않는다(2026-10-19).
6. **실수가 나면** 그 작업의 규칙 파일에 한 줄 더한다(여기 CLAUDE.md 에 쌓지 않는다 — 200줄 이내 유지).

**규칙 목록** (`.claude/rules/` 는 그 파일을 만질 때, 폴더 `CLAUDE.md` 는 그 폴더를 열 때 자동으로 읽힌다)

| 파일 | 언제 | 내용 |
|---|---|---|
| `.claude/rules/deploy.md` | `Code.gs`·워크플로 | 배포 절차·트리거 재설치·버전 상한·runMaint·/exec 403 |
| `.claude/rules/backend.md` | `Code.gs`·앱·공개 페이지 | 시트 ID·액션 목록·공개 액션·탭 |
| `.claude/rules/screens.md` | `portfolio.html`·`m.html`·`calc.js` | PC↔모바일 동시·통화·회색·계좌 그래프·외부입금·숨김 |
| `.claude/rules/public-page.md` | `dist_notice.html`·분배 파서 | 앱 먼저·실데이터로 이번 달+지난달 표·smoke 통과 |
| `.claude/rules/widget.md` | `tools/ticker*`·`ticker.html` | PC 위젯↔폰 짝·공지 받는 순서·시험 방법 |
| `watchdog/CLAUDE.md` · `mods/jjk-guard/CLAUDE.md` · `desktop-cat/CLAUDE.md` | 그 폴더 | 각 별도 프로젝트 |

## What this is

A single-file personal dividend-portfolio tracker (`portfolio.html`) for two people (재남 / 은경) holding mostly monthly-distribution Korean ETFs across several brokerage accounts. It tracks holdings, valuations, and monthly dividend income, and visualizes progress toward a goal (₩10,000,000/month in distributions by 2029-02-28).

There is **no build system, package manager, test suite, or lint config**. The frontend — HTML, CSS (`<style>`), and JS (`<script>`) — lives in `portfolio.html` (one large file). Open the file in a browser to run it; there is nothing to compile. The backend lives in `Code.gs` (Google Apps Script).

## Working in this repo

- **PDF·워드·파워포인트·엑셀은 읽기 전에 마크다운으로 바꾼다**(2026-10-06 사용자 결정 — 토큰 절약). `python -m markitdown 파일.pdf -o 파일.md` 로 바꾼 뒤(⚠️ `>` 금지 — 윈도우 PowerShell 은 `>` 로 받으면 UTF-16 으로 저장하고 한글이 cp949 를 거쳐 깨질 수 있다. `-o` 는 UTF-8 로 바로 쓴다) `.md` 를 읽는다. PDF 를 그대로 읽으면 쪽마다 이미지+글자로 두 번 읽혀 토큰이 이중으로 든다. **차트·그림이 핵심인 문서만** 원본을 본다. `python -c "import markitdown"` 가 실패하면 **묻지 말고** `python -m pip install --user "markitdown[all]"` 로 설치한다(파이썬 3.10↑, 원격 컨테이너 약 10초). 학교·집 PC 모두 처음 쓸 때 그 세션이 설치한다.
- **한글(HWP·HWPX) 문서는 `kordoc` 으로 읽고 쓴다**(2026-10-07 사용자 결정 — 정보부장 문서 작성용). 읽기 `kordoc 문서.hwp -o 문서.md`, 새로 쓰기 `kordoc generate 초안.md --preset 보고서 -o 보고서.hwpx`, 기안문 `kordoc fill --template gian -j 값.json -o 기안문.hwpx`. 결과는 `.hwpx`(한글 2014↑). `kordoc --version` 이 실패하면 **묻지 말고** `npm i -g kordoc` 로 설치한다(Node 필요 — 원격·학교·집 모두 그 세션이 설치). ⛔ 학생 개인정보가 든 문서는 이 공개 저장소에 올리지 않는다.

- **Editing**: The file is large with inline styles and one big script block. Use Grep to locate a function/section by line before editing rather than reading the whole file. Function definitions are plain `function name()` / `async function name()` at column 0, so `^(async )?function <name>` finds them fast. 파일 읽기 도구의 `allow_large`(큰 파일 통째 읽기, 2026-10 추가)는 **쓰지 않는다** — `portfolio.html` 7천여 줄을 통째로 읽으면 토큰이 크게 든다(사용자 토큰 절약 원칙). 정말 통째로 봐야 하면 먼저 묻는다.
- **Preview**: Just open `portfolio.html` in a browser (the launch preview panel also renders it). No dev server.
- ⛔ **비밀번호·열쇠 값은 저장소에 절대 적지 않는다.** 일지에도 코드에도 `••••`로만 쓴다. `jjk`는 **공개 저장소**다 — 한 번 커밋하면 지난 기록에 영구히 남고, 나중에 지워도 완전히 안 지워진다. 실제 값은 Apps Script **스크립트 속성**(`APP_TOKEN`)에만 둔다. (2026-08-05에 적힌 진입 비밀번호가 3주간 공개돼 있었다 — WORKLOG 127.)
- **Commit/push**: Commits go straight to `main` and are pushed to `origin` (GitHub `jaenamking1-collab/jjk`). Git identity is set locally as `jaenamking1-collab <jaenamking1@gmail.com>` (not global — new clones must set it). The owner works across **two PCs (work / home)** and wants every change committed and pushed automatically without asking.
- **Two-PC routine**: This repo is edited from two machines. **The very first thing in every session — before reading anything, before `git pull`: run `git status`. If the tree is dirty, commit and push it immediately**, before any other action. A dirty tree at session start means *the previous session on this PC ended without committing* and that work is stranded — pulling first can turn it into a conflict, and reading first wastes the chance to rescue it. Then `git pull` so the other PC's work (and `WORKLOG.md` / `CLAUDE.md` rules) is present. **End every session by** adding one entry to the top of `~/claude-memory/jjk/WORKLOG.md` (private repo — never put it back in this public one), then `git add .` → `git commit` → `git push`. `WORKLOG.md` is the running cross-PC log; keep it current.
- **This is also automated — but don't rely on it alone.** `.claude/settings.json` (committed, so both PCs get it) holds two hooks: `SessionStart` rescues a dirty tree then pulls, and `Stop` commits + pushes at the end of every turn. Both are limited to `main` and skip mid-merge/rebase.

## 세션 연속성 (WORKLOG.md)

이 프로젝트는 두 대의 Windows PC(직장 / 집)에서 Claude Code로 번갈아 작업한다. 대화 맥락이 PC 간에 이어지지 않으므로 `WORKLOG.md`를 Git으로 동기화되는 공유 메모리로 사용한다.

⛔ **일지는 이 저장소에 없다 — 비공개 `claude-memory` 저장소의 `~/claude-memory/jjk/WORKLOG.md` 다**(2026-10-16 이전, 사용자 결정). 실명·금액이 들어 있어 공개 저장소에서 git 기록째 지웠다. 아래에서 `WORKLOG.md` 는 모두 그 파일을 말한다. 읽기·쓰기는 그 경로로 하고, 저장·푸시는 `claude-memory` 훅이 한다. `jjk/WORKLOG.md` 를 다시 만들지 마라(`.gitignore` 가 막지만). 원격 세션은 `/home/user/claude-memory` 로 clone 되므로 `ln -sfn /home/user/claude-memory ~/claude-memory` 를 해 둔다(훅이 `$HOME/claude-memory` 를 푸시한다).

- **세션 시작 시**: ⚠️ **`git status`가 맨 처음이다.** 더러우면 **읽기도 pull도 하기 전에** 먼저 커밋·푸시한다 — 이전 세션이 커밋 없이 끝났다는 뜻이고, 그 작업은 이 PC에만 있다. 그 다음 `git pull`, 그 다음 `WORKLOG.md`를 읽어 맥락과 "다음 할 일"을 파악한다.
- **다른 PC 대화 이어받기**: "집에서/학교에서 한 거 이어가봐"는 **다른 기기의 세션**을 뜻한다. `list_sessions`는 이 PC만 본다 — 그것만 보고 "없다"고 답하지 마라(2026-08-24 학교에서 그렇게 답했다가 지적받음). 먼저 `git fetch origin`으로 넘어온 커밋을 보고, `~/claude-memory/transcripts/INDEX.md`를 읽는다. 필요하면 `python ~/claude-memory/hooks/index_transcripts.py read <PC>/<파일> [검색어]`로 원문을 확인한다.
- **지금 있는 PC에서 할 수 있는 것만 안내한다.** 다른 PC에서 해야 할 일은 **`WORKLOG.md`의 "다음 할 일"에만 남기고, 대화에서 시키지 마라.** 사용자는 지금 그 PC 앞에 없다 — 실행할 수 없는 명령을 받으면 할 일 목록이 아니라 잡음이다(2026-08-26 학교에서 집 PC 명령을 세 번 줬다가 지적받음). 다른 PC 차례가 되면 그 PC의 세션이 WORKLOG를 읽고 알아서 꺼낸다.
- ⚠️ **원격(클라우드) 세션에서 할 일은 하나뿐이다: `add_repo`로 `jaenamking1-collab/claude-memory`를 붙이고 clone.** 원격 세션의 GitHub 접근은 `jjk` 하나로 묶여 있어 `SessionStart` 훅의 clone이 **조용히 실패**하기 때문이다(`>/dev/null`). 붙이기만 하면 **저장·푸시는 훅이 알아서 한다** — 손으로 `save`를 부르거나 `CLAUDE_MEMORY_PC`를 넘길 필요 없다(2026-08-28에 자동화, WORKLOG 128).
  - 컨테이너는 `USERPROFILE`이 없다는 것으로 자동 판별해 `transcripts/cloud/`에 남는다. 두 PC(윈도우)는 각자 폴더를 쓴다.
  - **부작용**: 저장소를 하나 더 붙이면 그 세션이 앱 목록에서 `jjk` 아래가 아니라 **'기타'로 잡힌다.** 기록을 남기는 값이 더 크므로 감수한다.
  - 반대 방향(읽기)은 이미 자동이다 — 집·학교 PC는 `SessionStart` 훅의 `index_transcripts.py brief`가 다른 PC 대화 목록을 세션 맥락에 넣어준다. 사용자가 명령어를 칠 필요 없다.
- **세션 종료 시**: `WORKLOG.md` 맨 위에 새 항목(날짜, 장소(직장/집/원격), 한 일, 다음 할 일)을 추가한다. 과거 항목은 수정하지 않는다.
- **갱신 후**: `git add .` → `git commit` → `git push`까지 자동으로 수행한다. 사용자가 "항상 커밋·푸시"를 요청했으므로 매번 확인하지 않는다.

## Architecture

**Frontend (`portfolio.html`)** ⇄ **Google Apps Script backend (`Code.gs`)**, a Google Sheets–backed web app.

- The backend URL is the `API` constant at the top of `portfolio.html`'s `<script>` (`https://script.google.com/macros/s/.../exec`). All persistence lives in Google Sheets behind it — this repo has no database.
- **배포·실행·진단(clasp, deploy.yml, maint/runMaint, 버전 상한, /exec 403)** → `.claude/rules/deploy.md` (Code.gs·워크플로를 만지면 자동으로 읽힌다).
- **시트 ID·액션 목록·공개 액션·탭 구성** → `.claude/rules/backend.md`.

### Conventions to preserve

- 통화 표시·글자 크기·회색 규칙·계좌 그래프·외부입금·숨김·IRP 자동입력·국내/해외 나눔 → `.claude/rules/screens.md` (portfolio.html·m.html·calc.js 를 만지면 자동).


## ⛔ 사용자에게 시키기 전에 — 먼저 해보고 말해라

**이 항목이 이 문서에서 제일 자주 어겨진 규칙이다.** 2026-09-03 하루에만 네 번 반복돼 사용자가 화를 냈다.

- **"못 한다 / 해주세요"는 실제로 시도해 본 뒤에만 말한다.** 안 해보고 넘겨짚은 '못 함'은 전부 틀렸다:
  - `script.google.com` 403 하나 보고 "구글이 막혀 배포 불가"라고 했다 → 실제로는 clasp 가 쓰는 `script.googleapis.com`·`oauth2.googleapis.com` 은 열려 있었고 **원격에서 배포가 됐다**(WORKLOG 141).
  - `clasp` 가 없다고 "PC에서 하세요"라고 했다 → **설치하면 그만이다**(이 규칙은 위에도 이미 적혀 있었는데 또 어겼다).
  - "PC에서 `git pull` 하세요"라고 했다 → **이 저장소가 곧 GitHub Pages**다. 푸시하면 자동 반영이고 사용자가 할 일은 없었다.
    - ⛔ **사용자는 앱을 웹 주소(GitHub Pages)로 본다 — 로컬 파일이 아니다**(2026-09-11 본인 확인). 그러니 프론트(`portfolio.html`·`m.html`·`dist_notice.html`)를 고쳐 푸시했으면 사용자가 할 일은 **브라우저 새로고침뿐**이다.
    - 이 규칙이 있는데도 2026-09-11 한 세션에서만 네 번 어겼다. `WORKLOG.md` 루틴에 "`git pull` 해야 화면에 반영된다"는 **틀린 기록**이 있었고 그걸 근거로 삼았기 때문이다 — 그 기록은 2026-09-11에 지웠다. 저장소 안 기록끼리 어긋나면 **사용자에게 확인하고 하나로 정리해라.** 확인 없이 한쪽을 골라 시키지 마라.
    - `git pull` 을 부탁해도 되는 경우는 **그 PC에서 작업·배포할 때뿐**이다(예: `clasp push` 전 `Code.gs` 동기화).
- **순서**: ① 직접 해본다 → ② 막히면 *무엇이* 왜 막혔는지 명령·응답 코드로 확인한다 → ③ 그래도 사람만 할 수 있는 것(구글 계정 '허용' 클릭, 브라우저 로그인)만 부탁한다. ③에 해당하는지 스스로 증거를 못 대면 아직 ①이 안 끝난 것이다.
- **사용자의 되물음("배포가 안 된다고?", "그거 왜 안 돼?")은 점검 지시다.** 그 자리에서 다시 확인하고 답한다 — 앞서 한 말을 반복하지 않는다.
- 부탁을 할 때는 **왜 나는 못 하는지**(막힌 호스트, 없는 자격증명 등 구체적 근거)를 한 줄로 같이 준다. 근거를 못 쓰겠으면 부탁하지 마라.
- ⛔ **사용자가 자동화를 자꾸 확인하는 건 성격이 아니라 내가 끝까지 안 해서다.** 다른 세션에서 사용자 성격을 묻자 "자동화 확인을 많이 한다"고 답했는데(2026-09-28 지적), 그 확인은 내가 만든 일이다 — 같은 날 고친 코드가 `main`에 없었고, 파서가 조용히 0건을 냈고, OCR 결제가 막힌 걸 사용자가 화면을 보고서야 알았다. 사용자가 묻기 전에 **검증 결과(무엇을 어떻게 확인했는지)를 먼저 내놓고**, 지켜봐야 할 것은 내가 점검을 예약한다.
- ⛔ **고쳤다고 말하려면 `main` 병합 + 배포까지 끝나 있어야 한다.** 작업 브랜치에만 커밋하고 세션을 끝내면 사용자 화면은 그대로다. 2026-09-28 TIGER '월중 지급일' 수정이 `project-thread-sz1358` 에만 남아 있어, 사용자가 "아까 고치라고 했잖아"를 다시 말해야 했다(WORKLOG 196). 세션을 끝내기 전에 `git log origin/main..HEAD` 가 비었는지, `Code.gs` 가 바뀌었으면 `deploy.yml` → `maint resetAllTriggers` 까지 돌았는지 확인한다.
- ⛔ **내가 연 것은 내가 닫는다.** 브라우저(Claude in Chrome 등)로 연 탭·탭 그룹, 띄운 로컬 서버는 **작업이 끝나면 그 세션에서 직접 닫는다.** 사용자에게 "그룹 닫기 하세요"라고 정리를 넘기지 마라(2026-09-28 지적: 크롬에 'Claude' 탭 그룹 8개와 죽은 localhost 탭이 쌓여 있었다). 브라우저 도구가 없는 세션(원격 컨테이너)이라 못 닫을 때만 그 근거를 대고 부탁한다.

## Working principles

Adapted from the [Karpathy coding guidelines](https://x.com/karpathy/status/2015883857489522876) — they matter more than usual here because there is no test suite or type checker to catch mistakes, and the backend is deployed separately from the frontend. They bias toward caution over speed; for trivial edits, use judgment.

1. **Think before coding.** State assumptions out loud instead of hiding uncertainty. When a request has multiple valid readings (e.g. "remove the ₩" — every page, or just totals?), lay out the options and recommend the simpler one before editing. If something is genuinely ambiguous, stop and ask rather than guess — a wrong guess here ships to a live personal-finance app.
2. **Simplicity first.** This is a personal two-user tool, not a framework. Write the minimal change that solves the actual request — no speculative features, config toggles, abstractions, or defensive handling for cases that can't occur. Match the existing plain-`function`, inline-style, `api()`/`apiPost()` idiom rather than introducing new patterns. *Self-check: "Would a senior engineer call this overcomplicated?" If 200 lines could be 50, rewrite it.*
3. **Surgical changes.** Edit only what the task needs. Don't reformat, rename, or "improve" untouched code in the same file — diffs are reviewed by eye against a single large file, so noise hides real changes. Remove imports/variables/functions that *your* change orphaned, but flag pre-existing dead code (like the stray top-level debug lines that were in `Code.gs`) instead of silently deleting it unless asked. Preserve the documented **Conventions** above. *The test: every changed line should trace directly to the request.*
4. **Goal-driven execution.** Define how you'll verify before you start, then loop until it holds. Reframe vague tasks as checkable goals: "fix the bug" → reproduce it first — there's no test runner, so reproduce in the browser console or a scratch fetch (e.g. dumping `_distData` to pin down a mis-render), then confirm the reproduction is gone. For multi-step work, state a brief plan with a verify step per line. Remember `Code.gs` changes take effect only after deploy (`deploy.yml` → `maint resetAllTriggers`), and state the concrete success criterion ("월합계 셀에 ₩가 사라지고 숫자 크기가 나머지와 같다") and confirm it's met.

## Files

- `portfolio.html`·`m.html`·`calc.js` — 앱(PC·모바일). **PC 를 고치면 같은 턴에 모바일도, 묻지 않는다.** 상세는 `.claude/rules/screens.md`.
- `dist_notice.html` — 공개 분배금공지 페이지(많은 사람이 본다). 상세는 `.claude/rules/public-page.md`. 주소 `https://jjk.distributionjn.workers.dev/`.
- `Code.gs` — the Google Apps Script backend (mirror of the deployed script; not auto-deployed).
- `okx_nft_alert.gs` — unrelated to the portfolio app: an OKX NFT (Kaia) Legendary/Mystic listing watcher that writes Google Calendar alerts. Mirror of a **separate** Apps Script project — do not merge it into `Code.gs` (its 30-min trigger would steal the portfolio backend's execution slots; see WORKLOG 85·86). Also not auto-deployed. Design: `docs/superpowers/specs/2026-08-08-okx-nft-legendary-alert-design.md`.
- `watchdog/`·`mods/jjk-guard/`·`desktop-cat/` — 각 폴더의 `CLAUDE.md` 에 규칙(그 폴더 파일을 열면 자동).
- `tools/ticker.pyw`·`ticker.html` — 시세 위젯(PC·폰 짝). 상세는 `.claude/rules/widget.md`.
- `WORKLOG.md` — running work log for syncing across the two PCs; append a dated entry each session. **Lives in the private `claude-memory` repo at `~/claude-memory/jjk/WORKLOG.md`**, not here.
- `README.md` — one line (`# jjk`); no other docs.
