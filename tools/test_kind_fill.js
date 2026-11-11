// 거래소 공시(KIND) 예비 길 자체 점검. 실행: node tools/test_kind_fill.js
// Code.gs 의 실제 fetchDist_plus 를 떼어, 한화 사이트·PC 사본이 옛 회차에 멈춘 상황을 먹여 본다.
const fs = require('fs'), assert = require('assert'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '..', 'Code.gs'), 'utf8');
const i = src.indexOf('function fetchDist_plus'), j = src.indexOf('\nfunction _fetchDistPlusSite', i);
assert.ok(i >= 0 && j > i, 'fetchDist_plus 를 못 찾음');
const kind = [   // 2026-11-11 실제 중계 응답(_kindRounds 모양)
  { cycle: '월중', std: '20261113', pub: '2026-11-11', record: '2026-11-13', pay: '2026-11-17',
    items: [{ ticker: '494420', name: 'PLUS 미국배당증가성장주데일리커버드콜', amount: 102 }, { ticker: '0132K0', name: 'PLUS 테슬라위클리커버드콜채권혼합', amount: 160 }] },
  { cycle: '월말', std: '20261030', pub: '2026-10-28', record: '2026-10-30', pay: '2026-11-03', items: [{ ticker: '152100', name: 'PLUS 200', amount: 665 }] },
];
const it = (cycle, base, ticker) => ({ ticker, name: 'x', amount: 1, rate: 1, cycle, sched: { '기준일': base } });
const run = site => new Function('_fetchDistPlusSite', '_kindRounds', '_isOffDay', 'getNavMap', 'console',
  src.slice(i, j) + '\nreturn fetchDist_plus();')(() => site, () => kind, d => d.getDay() % 6 === 0, () => ({ navs: { '494420': 10000 } }), console);

// ① 사이트가 10월 회차에 멈춤 → 월중만 공시로 바뀌고 월말(같은 회차)은 사이트 것 그대로
let r = run({ success: true, items: [it('월중', '10월 15일', 'a'), it('월말', '10월 30일', 'b')], schedule: {} });
assert.strictEqual(r._fallback, 'kind');
assert.deepStrictEqual(r.items.filter(x => x.cycle === '월중').map(x => x.ticker), ['494420', '0132K0']);
assert.deepStrictEqual(r.items.find(x => x.ticker === '494420').sched, { '공시일': '11월 11일', '분배락일': '11월 12일', '기준일': '11월 13일', '지급일': '11월 17일' });
assert.strictEqual(r.items.find(x => x.ticker === '494420').rate, 1.02);
assert.strictEqual(r.items.find(x => x.ticker === '0132K0').rate, null);
assert.deepStrictEqual(r.items.filter(x => x.cycle === '월말').map(x => x.ticker), ['b']);
// ② 사이트가 이미 같은 회차를 읽음 → 손대지 않는다(운용사 원본이 이긴다)
const ok = { success: true, items: [it('월중', '11월 13일', 'a'), it('월말', '10월 30일', 'b')], schedule: {} };
assert.strictEqual(run(ok), ok);
// ③ 사이트 통째 실패(0건) → 공시의 두 회차로
r = run({ items: [], error: 'PLUS: Address unavailable' });
assert.strictEqual(r.items.length, 3);
// ④ 해 넘김: 사이트가 1월 15일 회차를 갖고 있는데 공시는 12월 회차 → 사이트가 더 새것
kind[0] = { cycle: '월중', std: '20261215', pub: '2026-12-11', record: '2026-12-15', pay: '2026-12-17', items: [{ ticker: 'k', name: 'k', amount: 1 }] };
const jan = { success: true, items: [it('월중', '1월 15일', 'a'), it('월말', '10월 30일', 'b')], schedule: {} };
assert.strictEqual(run(jan), jan);
console.log('OK — 거래소 공시 예비 길 4가지 통과');
