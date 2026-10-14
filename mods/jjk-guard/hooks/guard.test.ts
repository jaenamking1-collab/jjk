import { test, expect } from 'claude-code/testing'
import { judgeCommand, judgeWrite, cacheLabel, turnLabel, weather } from './register'

const LIVE = 'AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s'

test('clasp deploy: -i 없으면 막고, 라이브 ID 면 통과', () => {
  expect(judgeCommand('clasp deploy -d "x"')).toBeDefined()
  expect(judgeCommand('clasp deploy -i AKfycbOTHER -d "x"')).toBeDefined()
  expect(judgeCommand(`clasp deploy -i ${LIVE} -d "x"`)).toBeUndefined()
})

test('clasp pull: 저장소 안이면 막고, 임시 폴더로 cd 하면 통과', () => {
  expect(judgeCommand('clasp pull')).toBeDefined()
  expect(judgeCommand('cd /tmp/chk && clasp pull')).toBeUndefined()
  expect(judgeCommand('Set-Location $env:TEMP\\chk; clasp pull')).toBeUndefined()
})

test('비밀값: 명령·파일에 토큰 모양이면 막고, 자격증명 파일은 통과', () => {
  const tok = 'ya29.' + 'a'.repeat(30)
  expect(judgeCommand(`echo ${tok} > note.txt`)).toBeDefined()
  expect(judgeCommand(`echo '{"token":"${tok}"}' > ~/.clasprc.json`)).toBeUndefined()
  expect(judgeWrite('/home/user/jjk/WORKLOG.md', 'key ghp_' + 'b'.repeat(36))).toBeDefined()
  expect(judgeWrite('/home/user/jjk/WORKLOG.md', 'APP_TOKEN 은 •••• 로 적는다')).toBeUndefined()
  expect(judgeCommand('git push -u origin main')).toBeUndefined()
})

test('tool.call: Bash 의 잘못된 deploy 는 엔진에서 거부된다', async $ => {
  const r = await $.tool.call({ tool: 'Bash', command: 'clasp deploy -d "x"' } as never)
  expect(String((r as { text?: string }).text ?? (r as { deny?: string }).deny)).toContain('jjk-guard')
})

test('session.measure: 상태줄에 컨텍스트·5시간·7일 사용량', async ($, on) => {
  const seen: (string | undefined)[] = []
  on('ui.status', (_$, e) => { seen.push(e.text); return { value: undefined } as never })
  on('ui.toast', () => ({ value: undefined }) as never)
  on('session.measure', (_$, e) => ({ changed: e.changed }))
  await $.session.measure({ context: { window: 200000, percent: 85 }, rateLimits: [{ kind: 'five_hour', percentUsed: 31 }, { kind: 'seven_day', percentUsed: 12 }], changed: ['context'] } as never)
  expect(seen.join('|')).toContain('⛈️ 컨텍스트 85% · 5시간 31% · 7일 12%')
})

test('캐시 남은 시간·이번 답 사용량 표시', () => {
  expect(cacheLabel(0, 5)).toBe('')
  expect(cacheLabel(1000, 1000 + 18 * 60000)).toBe('캐시 42분')
  expect(cacheLabel(1000, 1000 + 61 * 60000)).toBe('캐시 만료')
  expect(turnLabel(31, 33.5)).toBe('이번 답 +2.5%')
  expect(turnLabel(undefined, 33)).toBe('')
})

test('컨텍스트 날씨', () => {
  expect(weather(undefined)).toBe('')
  expect(weather(10)).toBe('☀️')
  expect(weather(45)).toBe('⛅')
  expect(weather(60)).toBe('☁️')
  expect(weather(80)).toBe('🌧️')
  expect(weather(90)).toBe('⛈️')
})
