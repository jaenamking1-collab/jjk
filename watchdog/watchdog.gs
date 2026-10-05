// ── 포트폴리오 백엔드 감시 (별도 Apps Script 프로젝트 '백엔드감시') ──
// 왜: 알림(카톡·캘린더)이 전부 본체(Code.gs) 안에서 나가서, 본체가 죽으면 '죽었다'는 알림도 못 나갔다.
//     2026-10-01 트리거 정지 → 2026-10-05 /exec 403 까지 알림 0건, 사용자가 화면을 보고 알았다.
// ⛔ Code.gs 와 합치지 마라 — 본체가 죽어도 이건 살아 있어야 한다.
// 알림은 **구글 캘린더 일정 1개**(사용자: 카톡은 알림이 안 울리고 캘린더는 울린다).
// ⛔ 자주 울리면 안 된다(사용자: "알림이 너무 많아져 버림") → **멈춤 한 번에 일정 하나.** 복구 알림 없음.
//    두 번 연속(30분 간격) 이상일 때만 울리고, 두 번 연속 정상이 돼야 다음 멈춤을 다시 알린다.
// 처음 한 번: 편집기에서 setup ▶ → 권한 허용. 그다음은 30분마다 watch 가 저절로 돈다.

const EXEC = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec';
const PAGE = 'https://jjk.distributionjn.workers.dev/';

function setup() {
  ScriptApp.getProjectTriggers().forEach(t => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('watch').timeBased().everyMinutes(30).create();
  CalendarApp.getDefaultCalendar().getName();   // 캘린더 권한을 지금 받아 둔다
  const r = probe();
  console.log('설치 완료 — 지금 상태: ' + (r.bad ? '⚠️ ' + r.title + ' · ' + r.detail : '정상 · ' + r.detail));
}

function watch() {
  const p = PropertiesService.getScriptProperties();
  const r = probe();
  p.setProperty('LAST_RUN', new Date().toISOString());
  if (!r.bad) {
    const ok = Number(p.getProperty('OK_N') || 0) + 1;
    p.setProperties({ OK_N: String(ok), BAD_N: '0' });
    if (ok >= 2) p.deleteProperty('ALERTED');
    return;
  }
  const bad = Number(p.getProperty('BAD_N') || 0) + 1;
  p.setProperties({ BAD_N: String(bad), OK_N: '0' });
  if (bad < 2 || p.getProperty('ALERTED')) return;
  const start = new Date(Date.now() + 60 * 1000), end = new Date(start.getTime() + 10 * 60 * 1000);
  CalendarApp.getDefaultCalendar().createEvent('🚨 ' + r.title, start, end, {
    description: r.detail + '\n\nClaude 에게 "백엔드 멈췄다"고 말해 주세요.\n분배금공지 페이지: ' + PAGE
  }).addPopupReminder(0);
  p.setProperty('ALERTED', '1');
}

// 이상이면 {bad:true,title,detail}. 기준은 .github/scripts/check_backend.py 와 같다.
function probe() {
  let res = null, err = '';
  for (let i = 0; i < 2 && !res; i++) {
    try {
      const h = UrlFetchApp.fetch(EXEC + '?action=getDistributionAll', { muteHttpExceptions: true, followRedirects: true });
      const j = h.getResponseCode() === 200 ? JSON.parse(h.getContentText()) : null;
      if (j && j.sources) res = j.sources; else err = 'HTTP ' + h.getResponseCode();
    } catch (e) { err = String(e).slice(0, 120); }
    if (!res && i === 0) Utilities.sleep(20000);
  }
  if (!res) return { bad: true, title: '포트폴리오 백엔드 응답 없음', detail: '백엔드 주소가 응답하지 않습니다 (' + err + ').' };
  const times = Object.keys(res).map(k => parseSaved(res[k] && res[k].savedAt)).filter(Boolean);
  if (!times.length) return { bad: true, title: '포트폴리오 백엔드 데이터 없음', detail: '응답은 왔지만 갱신 시각이 없습니다.' };
  const last = new Date(Math.max.apply(null, times));
  const hours = Math.floor((Date.now() - last) / 3600000);
  const day = Number(Utilities.formatDate(new Date(), 'Asia/Seoul', 'd'));
  const limit = (day >= 8 && day <= 12) || day >= 23 ? 6 : 30;   // 공지창이면 30분마다 돌아야 한다
  const when = Utilities.formatDate(last, 'Asia/Seoul', 'MM/dd HH:mm');
  if (hours >= limit) return { bad: true, title: '포트폴리오 백엔드 갱신 멈춤',
    detail: '분배 데이터가 ' + when + ' 이후 ' + hours + '시간째 갱신되지 않았습니다(기준 ' + limit + '시간). 자동 실행(트리거)이 멈췄을 수 있습니다.' };
  return { bad: false, detail: '마지막 갱신 ' + when + ' (' + hours + '시간 전)' };
}

// savedAt 은 'yyyy-MM-dd HH:mm'(KST) 또는 'Fri Sep 11 2026 11:59:00 GMT+0900 (...)' 두 형식으로 온다.
function parseSaved(v) {
  const s = String(v || '').trim();
  if (!s) return null;
  const m = s.match(/^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2})/);
  const d = m ? new Date(m[1] + 'T' + m[2] + ':00+09:00') : new Date(s.replace(/\s*\(.*\)$/, ''));
  return isNaN(d) ? null : d.getTime();
}
