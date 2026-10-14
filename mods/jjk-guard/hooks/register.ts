import type { Register } from 'claude-code'

// jjk CLAUDE.md 의 '하지 마라' 규칙 중 명령 문자열만 보고 판별되는 것을 실행 직전에 막는다.
// 글로 적어 둔 규칙은 여러 번 어겨졌다 — 여기서는 어기려는 순간 거부된다.

const LIVE_ID = 'AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s'

// 비밀값 모양: 구글 OAuth 토큰·리프레시 토큰·클라이언트 시크릿, GitHub·Anthropic 토큰
const SECRET = /ya29\.[\w-]{20,}|\b1\/\/0[\w-]{20,}|GOCSPX-[\w-]{10,}|\bghp_[A-Za-z0-9]{30,}|github_pat_\w{30,}|sk-ant-[\w-]{20,}|"refresh_token"\s*:\s*"[^"]{10,}"/
// 토큰을 담아도 되는 곳(저장소 밖 자격증명 파일)
const CRED_FILE = /\.clasprc\.json|[\\/]\.config[\\/]|[\\/]\.ssh[\\/]/

export function judgeCommand(cmd: string): string | undefined {
  // ① clasp deploy 는 반드시 라이브 배포 ID(-i) — 없으면 공개 /exec 주소가 새로 생긴다
  if (/\bclasp\s+deploy\b/.test(cmd) && !/\s(-i|--deploymentId)\s/.test(cmd + ' '))
    return 'clasp deploy 에 -i <라이브 배포 ID> 가 없다 — URL 이 새로 생긴다. `clasp deploy -i ' + LIVE_ID + ' -d "<note>"`'
  if (/\bclasp\s+deploy\b/.test(cmd) && !cmd.includes(LIVE_ID))
    return 'clasp deploy -i 의 ID 가 라이브 배포 ID 가 아니다 (' + LIVE_ID + ')'
  // ② 저장소 안 clasp pull 금지 — 로컬 Code.gs 가 옛 원격본으로 덮인다. 임시 폴더로 cd 한 뒤만 허용
  if (/\bclasp\s+pull\b/.test(cmd) && !/\b(cd|Set-Location|pushd)\s+["']?[^"'\s;&|]*(te?mp|scratch)/i.test(cmd))
    return '저장소 안에서 clasp pull 금지 — 임시 폴더에 .clasp.json 만 복사해 cd 한 뒤 pull 한다'
  // ③ 명령에 비밀값이 그대로 들어 있으면(커밋·로그로 샐 수 있다) 막는다. 자격증명 파일에 쓰는 것만 예외
  if (SECRET.test(cmd) && !CRED_FILE.test(cmd))
    return '명령에 토큰·비밀값 모양 문자열이 있다 — jjk 는 공개 저장소다. 값은 스크립트 속성·Secret 에만 둔다'
  return undefined
}

export function judgeWrite(path: string, text: string): string | undefined {
  if (SECRET.test(text) && !CRED_FILE.test(path))
    return path + ' 에 토큰·비밀값 모양 문자열을 쓰려 한다 — jjk 는 공개 저장소다. `••••` 로 적는다'
  return undefined
}

// 배포를 돌렸는데 resetAllTriggers 를 아직 안 돌렸으면 상태줄에 계속 띄운다. 사용량도 같은 줄에.
let deployPending = false
let usageText = ''
const warned: Record<string, number> = {}
// Token Weather 처럼: 프롬프트 캐시 남은 시간(마지막 응답 뒤 1시간), 이번 답이 쓴 5시간 한도
const CACHE_MS = 60 * 60 * 1000
let lastReplyAt = 0, nowMs = 0, fiveNow: number | undefined, fiveAtPrompt: number | undefined, turnText = ''
export function cacheLabel(lastAt: number, now: number): string {
  if (!lastAt) return ''
  const left = Math.ceil((lastAt + CACHE_MS - now) / 60000)
  return left > 0 ? '캐시 ' + left + '분' : '캐시 만료'
}
export function turnLabel(before?: number, after?: number): string {
  if (before === undefined || after === undefined) return ''
  const d = Math.round((after - before) * 10) / 10
  return '이번 답 ' + (d > 0 ? '+' : '') + d + '%'
}
function paint($: { ui: { status: (t?: string) => unknown } }) {
  const parts = [deployPending ? '⚠ 배포 뒤 resetAllTriggers 안 함' : '', usageText, turnText, cacheLabel(lastReplyAt, nowMs)].filter(Boolean)
  $.ui.status(parts.length ? parts.join(' · ') : undefined)
}

export const register: Register = on => {

  on('tool.call', ($, e, next) => {
    const a = e as unknown as Record<string, unknown>
    let why: string | undefined
    if (e.tool === 'Bash' || e.tool === 'PowerShell') why = judgeCommand(String(a.command ?? ''))
    else if (e.tool === 'Write') why = judgeWrite(String(a.file_path ?? ''), String(a.content ?? ''))
    else if (e.tool === 'Edit') why = judgeWrite(String(a.file_path ?? ''), String(a.new_string ?? ''))
    if (why) return { deny: 'jjk-guard: ' + why }

    // ④ deploy.yml 을 돌리면 '트리거 재설치 대기' 표시, maint resetAllTriggers 를 돌리면 해제
    if (/actions_run_trigger$/.test(e.tool) && a.method === 'run_workflow') {
      const wf = String(a.workflow_id ?? ''), fn = String((a.inputs as Record<string, unknown> | undefined)?.fn ?? '')
      if (/^deploy\.ya?ml$/.test(wf)) {
        deployPending = true
        paint($)
      } else if (/^maint\.ya?ml$/.test(wf) && fn === 'resetAllTriggers') {
        deployPending = false
        paint($)
      }
    }
    return next(e)
  }).catch(($, e, next) => (next.called ? next(e) : { deny: 'jjk-guard: 검사 중 오류 — 안전을 위해 막았다' }))

  on('session.start', async ($, e, next) => {
    nowMs = await $.clock.now()
    $.clock.every(60000, async () => { nowMs = await $.clock.now(); paint($) })
    return next(e)
  })

  on('prompt.submit', ($, e, next) => {
    fiveAtPrompt = fiveNow
    return next(e)
  }).catch(($, e, next) => next(e))

  on('turn.complete', async ($, e, next) => {
    lastReplyAt = nowMs = await $.clock.now()
    turnText = turnLabel(fiveAtPrompt, fiveNow)
    paint($)
    if (deployPending) $.ui.toast('jjk-guard: 배포했는데 resetAllTriggers 를 안 돌렸다 — 트리거가 조용히 죽는다')
    return next(e)
  })

  // ⑤ 사용량: 컨텍스트·5시간·7일 한도를 상태줄에, 80%·90% 를 처음 넘을 때 한 번씩 알림
  on('session.measure', ($, e, next) => {
    const lim = (k: string) => e.rateLimits.find(r => r.kind === k)
    const pct = (v?: number) => (v === undefined ? '-' : Math.round(v) + '%')
    const f = lim('five_hour'), w = lim('seven_day')
    fiveNow = f?.percentUsed
    usageText = '컨텍스트 ' + pct(e.context.percent) + ' · 5시간 ' + pct(f?.percentUsed) + ' · 7일 ' + pct(w?.percentUsed)
    paint($)
    for (const [name, v] of [['컨텍스트', e.context.percent], ['5시간 한도', f?.percentUsed], ['7일 한도', w?.percentUsed]] as const) {
      const step = v === undefined ? 0 : v >= 90 ? 90 : v >= 80 ? 80 : 0
      if (step > (warned[name] ?? 0)) $.ui.toast('jjk-guard: ' + name + ' ' + Math.round(v as number) + '% 사용')
      warned[name] = step
    }
    return next(e)
  })
}
