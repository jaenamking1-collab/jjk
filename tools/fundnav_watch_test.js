// IRP 기준가 감시(_fundNavMissedBizDays)가 주말·명절 뒤에 오경보를 내지 않는지 본다. 실행: node tools/fundnav_watch_test.js
process.env.TZ = 'Asia/Seoul';
const src = require('fs').readFileSync(__dirname + '/../Code.gs', 'utf8').replace(/\r/g, '');
const pick = (a, b) => src.slice(src.indexOf(a), src.indexOf(b, src.indexOf(a)));
eval(pick('const KR_HOLIDAYS', '// 그 달의 마지막 영업일') + pick('function _fundNavMissedBizDays', '\n}\n') + '\n}');
let bad = 0;
[['2026-10-16', '2026-10-19', false],   // 금 → 월(2026-10-19 오경보 난 날)
 ['2026-10-16', '2026-10-20', false],   // 월 하루 빔
 ['2026-10-16', '2026-10-21', true],    // 월·화 빔 → 경보
 ['2026-09-23', '2026-09-29', false],   // 추석 휴장 뒤
 ['2026-10-08', '2026-10-12', false],   // 한글날(금) 뒤 월
 ['2026-10-02', '2026-10-06', false],   // 개천절 대체(월) 뒤 화
 ['2026-10-14', '2026-10-17', false],   // 토요일은 안 울림
 ['2026-10-12', '2026-10-15', true]].forEach(([a, b, want]) => {
  const alert = _fundNavMissedBizDays(a, b) >= 2;
  if (alert !== want) { bad++; console.log('FAIL', a, '->', b, '경보', alert); }
});
console.log(bad ? bad + '건 실패' : '전부 통과');
process.exit(bad ? 1 : 0);
