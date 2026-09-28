// 분배금 예측 B 방식(최근 3회 1주당 × 지금 수량) 자체 점검. 실행: node tools/test_perpay.js
// portfolio.html·m.html 안의 실제 코드를 그대로 떼어 돌린다(복사본을 따로 두면 원본과 어긋난다).
const fs = require('fs'), assert = require('assert'), path = require('path');
const read = f => fs.readFileSync(path.join(__dirname, '..', f), 'utf8');
const grab = (src, from) => {
  const i = src.indexOf(from), j = src.indexOf('\n}\n', i);
  assert.ok(i >= 0 && j > i, '코드 조각을 못 찾음: ' + from);
  return src.slice(i, j + 2);
};
const pc = read('portfolio.html'), mo = read('m.html');
const fnPc = grab(pc, 'function perPayEstimate('), fnMo = grab(mo, 'function perPayEstimate(');
assert.strictEqual(fnPc, fnMo, 'portfolio.html 과 m.html 의 perPayEstimate 가 다르다');
const perPay = new Function(fnPc + '\nreturn perPayEstimate;')();

// portfolio.html projectAnnualDist 도 통째로 돌려 본다(4번째 인자 = 원금월별).
const projPc = new Function(fnPc + grab(pc, 'function projectAnnualDist(') + '\nreturn projectAnnualDist;')();
// m.html 은 costFor(year) 로 원금월별을 고른다 — 올해만 쓴다.
const Y = new Date().getFullYear();
const projMo = cost => new Function('D', 'localStorage', fnMo + grab(mo, 'function costFor(')
  + grab(mo, 'function projectAnnualDist(') + '\nreturn projectAnnualDist;')({ cost }, { getItem: () => null });

const h = { id: 'h1', quantity: 300, div_cycle: '월' };
// 7·8·9월에 100주 → 200주 → 300주로 늘며 매번 주당 10원을 받았다.
const hist = { [Y]: { 7: 1000, 8: 2000, 9: 3000 } };
const cost = { h1_7: { qty: 100 }, h1_8: { qty: 200 }, h1_9: { qty: 300 } };

// ① B: 주당 10 × 300주 = 3000. (A 였다면 금액 평균 2000 으로 낮게 잡힌다)
assert.strictEqual(perPay(h, hist, Y, cost), 3000);
assert.strictEqual(perPay(h, hist, String(Y), cost), 3000, '연도가 문자열로 와도 같아야 한다');
assert.strictEqual(perPay(h, hist, Y, null), 2000, '원금월별이 없으면 A(최근 3회 금액 평균)');

// ② 세 회차 중 하나라도 수량 이력이 없으면 A — 지금 수량으로 과거를 나누면 주당이 틀린다.
assert.strictEqual(perPay(h, hist, Y, { h1_8: { qty: 200 }, h1_9: { qty: 300 } }), 2000);
assert.strictEqual(perPay(h, hist, Y, { ...cost, h1_7: { qty: 0 } }), 2000, '0주 행도 이력 없음');

// ③ 최근 3회만 본다 — 3월의 옛 소액은 안 섞인다.
assert.strictEqual(perPay(h, { [Y]: { 3: 5, 7: 1000, 8: 2000, 9: 3000 } }, Y, cost), 3000);

// ④ 올해 배당이 없으면 지난 해 값으로 A (올해 원금월별로 지난 해를 나누지 않는다).
assert.strictEqual(perPay(h, { [Y - 1]: { 10: 600, 11: 900, 12: 1200 } }, Y, { h1_10: { qty: 1 }, h1_11: { qty: 1 }, h1_12: { qty: 1 } }), 900);

// ⑤ 이력 없음 → 0, 판 종목 → A 그대로(투영 쪽이 따로 0 처리).
assert.strictEqual(perPay(h, {}, Y, cost), 0);
assert.strictEqual(perPay({ ...h, quantity: 0 }, hist, Y, cost), 2000);

// ⑥ 연 투영: 확정 3회(6000) + 남은 9개월 × 3000 = 33000. 두 화면이 같은 값을 내야 한다.
const divs = Object.entries(hist[Y]).map(([m, a]) => ({ holding_id: 'h1', year: Y, month: m, amount: a }));
assert.strictEqual(projPc([h], divs, Y, cost).h1, 6000 + 9 * 3000);
assert.strictEqual(projMo(cost)([h], divs, Y).h1, 6000 + 9 * 3000, 'm.html 투영이 portfolio.html 과 다르다');
assert.strictEqual(projPc([h], divs, Y, null).h1, 6000 + 9 * 2000);
assert.strictEqual(projMo(null)([h], divs, Y).h1, 6000 + 9 * 2000);
// 지난 해 화면은 원금월별을 안 쓴다(m.html costFor).
assert.strictEqual(projMo(cost)([h], divs.map(d => ({ ...d, year: Y - 1 })), Y - 1).h1, 6000 + 9 * 2000);

console.log('OK — 분배금 예측 B 방식 15건 통과 (두 파일 결과 동일)');
