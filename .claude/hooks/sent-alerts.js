// UserPromptSubmit 훅: 사용자가 받은 알림(텔레그램·카톡·메일)을 말하면, 우리가 보낸 원문이 어디 남는지 Claude 에게 바로 알려준다.
// 규칙 파일에 적어 둬도 그 순간 떠올리지 못해 텔레그램만 뒤졌다(2026-11-23 사용자: "맨날 규칙에 적어두고 알아서 못 찾는 게 문제").
let raw = '';
process.stdin.on('data', d => (raw += d));
process.stdin.on('end', () => {
  let p = '';
  try { p = JSON.parse(raw).prompt || ''; } catch (e) { return; }
  if (!/텔레그램|텔레|카톡|카카오|알림|메세지|메시지|메일|경고|날라온|날아온|온 거|왔던/.test(p)) return;
  process.stdout.write(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: 'UserPromptSubmit',
      additionalContext:
        '[보낸 알림 원문] 사용자가 받은 텔레그램·카톡·메일은 전부 우리 코드가 보낸 것이다. 추측·텔레그램 접속 말고 이것부터: ' +
        '① 드라이브 커넥터 read_file_content 로 비공개 시트 \'jjk 알림 발송기록\'(fileId 1QPiIeAgr8-HQOUs_aWtC221gL94LBp8i9lO5H4X3Leg, 맨 위가 최신) — Code.gs·Actions(runMaint sendKakaoMemo)·메일 전부 남는다(2026-11-23부터). ' +
        '② 그 전 것이나 시트에 없으면 그 시각의 실패한 GitHub Actions 실행 로그(smoke-ticker·smoke-public·script-versions 등, git credential fill 토큰으로 REST). ' +
        '③ 2026-11-23 이전 텔레그램은 maint _tgReadBack(arg=며칠) 로 되읽어 시트에 채운다. 원문을 찾은 뒤에 답한다.'
    }
  }));
});
