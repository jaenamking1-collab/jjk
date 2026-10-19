// UserPromptSubmit 훅: 사용자가 디자인을 말하면 디자인 스킬을 부르라고 Claude 에게 알려준다.
// 사용자가 /frontend-design·/impeccable 같은 명령어를 기억하지 않아도 되게 하려는 것(2026-10-19 사용자 요청).
let raw = '';
process.stdin.on('data', d => (raw += d));
process.stdin.on('end', () => {
  let p = '';
  try { p = JSON.parse(raw).prompt || ''; } catch (e) { return; }
  if (/^\s*\/(frontend-design|impeccable)\b/.test(p)) return; // 이미 직접 불렀으면 그대로
  const re = /디자인|레이아웃|UI|UX|예쁘게|이쁘게|꾸며|꾸미|세련|촌스|밋밋|색감|색깔|색상|배색|글꼴|폰트|여백|정렬이|화면.{0,6}(만들|바꿔|고쳐|다듬|개선|짜|구성)|(페이지|사이트|앱|대시보드|첫 ?화면|랜딩).{0,6}(만들|디자인|꾸며|다듬)/i;
  if (!re.test(p)) return;
  process.stdout.write(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: 'UserPromptSubmit',
      additionalContext:
        '[디자인 자동 연결] 이 요청이 화면·UI 디자인 작업이면, 작업 전에 Skill 도구로 디자인 스킬을 먼저 부른다: ' +
        '새 화면·페이지를 만들면 frontend-design, 이미 있는 화면을 점검·다듬·고치면 impeccable. ' +
        '사용자는 명령어를 외우지 않기로 했으니 묻지 말고 부르고, 어느 스킬을 썼는지 한 줄로 알린다. ' +
        '디자인 작업이 아니면(예: 원본↔화면 값 대조) 무시한다.'
    }
  }));
});
