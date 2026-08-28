// 달력 '회차 상태 한 줄' 자체 점검. 실행: node tools/test_calstatus.js
// portfolio.html 과 dist_notice.html 의 실제 코드를 각각 떼어내 같은 자료를 먹이고 결과가 같은지 본다
// (CLAUDE.md: 두 파일에 중복된 계산은 한쪽만 고치면 조용히 어긋난다).
const fs = require('fs'), assert = require('assert'), path = require('path');
const grab = (file) => {
  const src = fs.readFileSync(path.join(__dirname, '..', file), 'utf8');
  const i = src.indexOf('  const _md = k =>'), j = src.indexOf('})();', i);
  assert.ok(i >= 0 && j > i, file + ' 에서 상태 줄 코드를 못 찾음');
  return src.slice(i, j + 5);
};
// 2026년 8월 말 회차 실제 데이터: 분배락 8/28(금), 기준 8/31(월), 지급 9/2(수)
const marks = {
  '8-26': [{ label: '분배금공시일' }], '8-28': [{ label: '분배락일' }],
  '8-31': [{ label: '분배금지급기준일' }], '9-2': [{ label: '월말 지급일' }],
};
const prevBizDay = (m, d) => {                       // 8/28 직전 영업일 = 8/27
  const dt = new Date(2026, m - 1, d);
  do { dt.setDate(dt.getDate() - 1); } while (dt.getDay() === 0 || dt.getDay() === 6);
  return { m: dt.getMonth() + 1, d: dt.getDate() };
};
const run = (code, today) => new Function('marks', 'year', 'month', 'today', 'prevBizDay',
  code + '\nreturn statusHtml;')(marks, 2026, 8, today, prevBizDay);
const A = grab('portfolio.html'), B = grab('dist_notice.html');
const cases = [
  [28, /오늘 8\/28\(금\) 분배락일/, /8\/27\(목\)<\/b>로 마감/],   // 분배락 당일
  [20, /다음 매수 마감 <b>8\/27\(목\)<\/b> <b>D-7<\/b>/, /사면 받습니다/],
  [27, /<b>오늘까지<\/b>/, null],                                  // 마감 당일은 D-0 대신 '오늘까지'
  [29, /이번 회차 매수는 끝났습니다/, /지급 <b>9\/2\(수\)<\/b> 예정/], // 회차 사이
];
for (const [day, must, must2] of cases) {
  const a = run(A, day), b = run(B, day);
  assert.strictEqual(a, b, `${day}일: 두 파일 결과가 다르다\n  앱 : ${a}\n  공지: ${b}`);
  assert.match(a, must, `${day}일 문구: ${a}`);
  if (must2) assert.match(a, must2, `${day}일 문구: ${a}`);
}
// 마감은 지났는데 분배락은 아직인 주말 구간(분배락 8/31 월 → 마감 8/28 금, 오늘 8/29 토)
const weekend = { '8-31': [{ label: '분배락일' }], '9-1': [{ label: '분배금지급기준일' }] };
const wa = new Function('marks','year','month','today','prevBizDay', A + '\nreturn statusHtml;')(weekend, 2026, 8, 29, prevBizDay);
const wb = new Function('marks','year','month','today','prevBizDay', B + '\nreturn statusHtml;')(weekend, 2026, 8, 29, prevBizDay);
assert.strictEqual(wa, wb, '주말 구간에서 두 파일 결과가 다르다');
assert.match(wa, /매수는 <b>8\/28\(금\)<\/b>로 마감됐습니다/, wa);
// 27일은 D-0 이 아니라 '오늘까지' 로 나와야 한다(D-0 은 읽는 사람이 헷갈린다)
assert.ok(!/D-0/.test(run(A, 27)), 'D-0 이 그대로 노출되면 안 된다');
// 회차 사이: 앞으로 올 분배락이 없으면 날짜를 만들어내지 않고 지급 예정만 알린다
const onlyPay = { '9-2': [{ label: '월말 지급일' }] };
const late = new Function('marks','year','month','today','prevBizDay', A + '\nreturn statusHtml;')(onlyPay, 2026, 8, 30, prevBizDay);
assert.match(late, /이번 회차 매수는 끝났습니다 · 지급 <b>9\/2\(수\)<\/b> 예정/, late);
assert.ok(!/D-/.test(late), '없는 회차의 D-day 를 만들어내면 안 된다');
console.log('OK — 회차 상태 줄 ' + (cases.length * 3 + 4) + '건 통과 (두 파일 결과 동일)');
