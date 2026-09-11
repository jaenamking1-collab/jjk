// 외부 '주식상황/분배금' 스프레드시트(19UsD0Tz…)에 붙어 있는 **별개** Apps Script 프로젝트의 미러.
// ⛔ Code.gs(포트폴리오관리)와 합치지 마라 — 다른 프로젝트다. .claspignore 가 push 에서 막는다.
// 저장소에 두는 이유는 하나다: 여기 없으면 이 코드가 어디에도 기록되지 않는다.

// ── 분배금 입력칸 자동 하이라이트 ──────────────────
// 매일 07시(KST): 오늘이 어떤 회차의 지급일 ±2일이면, 그 회차 행의 '지급월' 칸을 노랗게 칠한다.
// → 분배금이 들어온 달을 손으로 찾지 않고 바로 입력할 수 있다.
const DIV_SHEET_ID = '19UsD0Tz6YL2eDoLdocL0ify8NLbUYSHaOOV-jtDqNLU';
const DIV_HL = '#ffff00';
const DIST_EXEC = 'https://script.google.com/macros/s/AKfycbwJS1Fd-sDCVKPLJEpEWZmPQEKAOR9pG7y-nPKZOYty65j3ArOmlDzNX2WFqiGNF_s/exec';

function _parseMD(s) {
  const m = String(s || '').match(/(\d{1,2})월\s*(\d{1,2})일/);
  return m ? { mon: +m[1], day: +m[2] } : null;
}

// 오늘(KST)과 (월,일)의 최소 일수차. 전/후년도까지 보는 건 연말·연초 랩 때문이다
// (12월 31일 지급을 1월 2일에 보면 -364일이 아니라 -2일이어야 한다).
function _diffDays(mon, day) {
  const ts = Utilities.formatDate(new Date(), 'Asia/Seoul', 'yyyy-MM-dd');
  const t = Date.parse(ts + 'T00:00:00Z'), y0 = +ts.slice(0, 4);
  let best = 9999;
  [y0 - 1, y0, y0 + 1].forEach(y => {
    const m = Date.parse(y + '-' + ('0' + mon).slice(-2) + '-' + ('0' + day).slice(-2) + 'T00:00:00Z');
    if (isNaN(m)) return;
    const d = Math.round((m - t) / 86400000);
    if (Math.abs(d) < Math.abs(best)) best = d;
  });
  return best;
}

function markDivInputCells() {
  try {
    // ⚠️ 예전엔 운용사마다 getDistribution 을 불러 **6회** 요청했다. 앱에 6개사를 한 번에 주는
    // getDistributionAll 이 있다. Apps Script 는 같은 계정 실행을 직렬화하므로, 6회 호출은
    // 백엔드 슬롯을 그만큼 오래 물어 다른 요청을 줄세운다(2026-09-10 전면 정지 사고 참고).
    const cycleMonth = {};
    try {
      const res = UrlFetchApp.fetch(DIST_EXEC + '?action=getDistributionAll', { muteHttpExceptions: true });
      if (res.getResponseCode() !== 200) { console.log('HTTP', res.getResponseCode()); return; }
      const sources = (JSON.parse(res.getContentText('UTF-8')) || {}).sources || {};
      Object.keys(sources).forEach(src => {
        ((sources[src] || {}).items || []).forEach(it => {
          if (it.hist) return;                                   // 과거 회차 병합분 제외
          const md = _parseMD(it.sched && it.sched['지급일']);
          if (!md) return;
          let cyc = it.cycle;
          if (cyc !== '월중' && cyc !== '월말') cyc = (md.day >= 25 ? '월말' : '월중');
          if (Math.abs(_diffDays(md.mon, md.day)) <= 2) cycleMonth[cyc] = md.mon;
        });
      });
    } catch (e) { console.log('분배 조회 실패', e); return; }
    if (!Object.keys(cycleMonth).length) return;                 // 오늘 지급일인 회차가 없으면 끝

    const sh = SpreadsheetApp.openById(DIV_SHEET_ID).getSheetByName('분배금');
    if (!sh) { console.log('분배금 시트 없음'); return; }
    const n = sh.getLastRow() - 4;
    if (n < 1) return;

    const cyc = sh.getRange(5, 4, n, 1).getValues();             // D열 회차(월중/월말)
    const tkr = sh.getRange(5, 2, n, 1).getValues();             // B열 티커
    const rng = sh.getRange(5, 7, n, 12);                        // G~R열 = 1~12월
    const bg = rng.getBackgrounds();

    for (let i = 0; i < n; i++) {
      const c = String(cyc[i][0] || '').trim();
      if (c !== '월중' && c !== '월말') continue;
      if (/^[A-Za-z]/.test(String(tkr[i][0] || '').trim())) continue;   // 해외주식(영문 티커) 제외
      // ⚠️ 지난 달 하이라이트를 먼저 지운다. 예전엔 `if(!bg[i][m])` 로 **빈 칸만** 흰색으로 칠해서
      // 이미 노란 칸은 그대로 남았다 → 달마다 노랑이 쌓였다. 내가 칠한 노랑만 골라 지운다.
      for (let m = 0; m < 12; m++) if (String(bg[i][m]).toLowerCase() === DIV_HL) bg[i][m] = '#ffffff';
      const tm = cycleMonth[c];
      if (tm) bg[i][tm - 1] = DIV_HL;
    }
    rng.setBackgrounds(bg);
  } catch (e) { console.log('markDivInputCells 실패', e, e && e.stack); }
}

// ── 분배금 입력 시 그때의 보유수량을 '수량스냅'에 남긴다 ──
// 나중에 "그 달에 몇 주 갖고 받은 분배금인지"를 알기 위한 기록. 주당 분배금 계산의 근거가 된다.
const CFG = {
  divSheet: '분배금', qtySheet: '주식상황', snapSheet: '수량스냅',
  qtyCol: 4, firstDivCol: 7, lastDivCol: 18, firstRow: 5
};
function onEdit(e) {
  const sh = e.range.getSheet();
  if (sh.getName() !== CFG.divSheet) return;
  const ss = e.source;
  const qtySh = ss.getSheetByName(CFG.qtySheet);
  let snap = ss.getSheetByName(CFG.snapSheet);
  if (!snap) snap = ss.insertSheet(CFG.snapSheet).hideSheet();
  const r0 = e.range.getRow(), c0 = e.range.getColumn();
  const nR = e.range.getNumRows(), nC = e.range.getNumColumns();
  for (let i = 0; i < nR; i++) {
    const r = r0 + i;
    if (r < CFG.firstRow) continue;
    const qty = qtySh.getRange(r, CFG.qtyCol).getValue();
    if (qty === '' || isNaN(qty)) continue;
    for (let j = 0; j < nC; j++) {
      const c = c0 + j;
      if (c < CFG.firstDivCol || c > CFG.lastDivCol) continue;
      const v = sh.getRange(r, c).getValue();
      if (v === '' || isNaN(v) || Number(v) <= 0) continue;
      snap.getRange(r, c).setValue(qty);
    }
  }
}

// ===== 수동 실행 =====
// 편집기에서 ▶ 1회 → 매일 07시(KST) 트리거 설치.
// ⚠️ 코드를 다시 올린 뒤에도 눌러라. 스코프가 바뀌면 기존 트리거가
// 'Authorization is required to perform that action.' 으로 죽고 재인증만으론 안 살아난다.
function installDivMarkTrigger() {
  ScriptApp.getProjectTriggers().forEach(t => {
    if (t.getHandlerFunction() === 'markDivInputCells') ScriptApp.deleteTrigger(t);
  });
  ScriptApp.newTrigger('markDivInputCells').timeBased().everyDays(1).atHour(7).create();
  return { ok: true };
}
