// 전달 대비 증감 셀 자체 점검. 실행: node tools/test_distchg.js
// dist_notice.html 안의 실제 코드를 그대로 떼어 돌린다(복사본을 따로 두면 원본과 어긋난다).
const fs = require('fs'), assert = require('assert'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '..', 'dist_notice.html'), 'utf8');
const grab = (from, to) => {
  const i = src.indexOf(from), j = src.indexOf(to, i);
  assert.ok(i >= 0 && j > i, '코드 조각을 못 찾음: ' + from);
  return src.slice(i, j);
};
const monthOf = (s) => { s = String(s || ''); let m = s.match(/(\d{1,2})월/); if (m) return parseInt(m[1]); m = s.match(/(\d{1,2})[\/\.]\d{1,2}/); if (m) return parseInt(m[1]); return null; };
const cycleOf = (it) => it.cycle || '';
const body = grab('const prevM = selM === 1', '    let body;');
const run = (selM, d, it, cyc) => new Function('selM', 'd', 'monthOf', 'cycleOf', 'it', 'cyc',
  body + '\nreturn chgHtml(it, cyc);')(selM, d, monthOf, cycleOf, it, cyc);

const d = { schedule: {}, items: [
  { ticker: 'A', cycle: '월말', amount: 100, sched: { 기준일: '7월 31일' }, hist: true },
  { ticker: 'B', cycle: '월말', amount: 200, sched: { 기준일: '7월 31일' }, hist: true },
  { ticker: 'A', cycle: '월중', amount: 999, sched: { 기준일: '7월 15일' }, hist: true },
  { ticker: 'C', cycle: '월말', amount: 500, sched: { 기준일: '6월 30일' }, hist: true }, // 전전달 — 쓰면 안 됨
]};
assert.match(run(8, d, { ticker: 'A', cycle: '월말', amount: 110 }, '월말'), /▲10\.0%/);
assert.match(run(8, d, { ticker: 'B', cycle: '월말', amount: 150 }, '월말'), /▼25\.0%/);
assert.match(run(8, d, { ticker: 'A', cycle: '월말', amount: 100 }, '월말'), /0\.0%/);
assert.ok(!/▲|▼/.test(run(8, d, { ticker: 'A', cycle: '월말', amount: 100 }, '월말')));
assert.match(run(8, d, { ticker: 'C', cycle: '월말', amount: 500 }, '월말'), />-</, '전전달 값을 끌어다 쓰면 안 된다');
assert.match(run(8, d, { ticker: 'Z', cycle: '월말', amount: 100 }, '월말'), />-</, '전달 데이터 없으면 -');
assert.match(run(8, d, { ticker: 'A', cycle: '월중', amount: 1998 }, '월중'), /▲100\.0%/, '주기가 다르면 섞이면 안 된다');
assert.match(run(1, { schedule: {}, items: [{ ticker: 'A', cycle: '월말', amount: 100, sched: { 기준일: '12월 31일' }, hist: true }] },
  { ticker: 'A', cycle: '월말', amount: 120 }, '월말'), /▲20\.0%/, '1월은 전달이 12월');
console.log('OK — 전달 대비 증감 8건 통과');
