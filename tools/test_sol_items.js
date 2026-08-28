// SOL 블로그 공지 표 파싱 자체 점검. 실행: node tools/test_sol_items.js
// Code.gs 의 실제 _solItems 를 그대로 떼어 돌린다. SOL은 회차마다 표기를 바꾸고,
// 바뀌면 그 회차가 통째로 사라지므로(0건이면 호출측이 버린다) 형식별로 못 박아 둔다.
const fs = require('fs'), assert = require('assert'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '..', 'Code.gs'), 'utf8');
const i = src.indexOf('function _solItems'), j = src.indexOf('\n// 회차 판정', i);
assert.ok(i >= 0 && j > i, '_solItems 를 못 찾음');
const _solItems = new Function(src.slice(i, j) + '\nreturn _solItems;')();

const head = '2026년 8월 분배금 안내 분배금 내역 NO ETF명 분배금(원) 분배율(%) ';
const same = (got, want, msg) => {
  assert.strictEqual(got.length, want.length, `${msg}: ${got.length}건 (기대 ${want.length}건)`);
  want.forEach((w, k) => {
    assert.strictEqual(got[k].name, w[0], `${msg} ${k}행 이름: ${got[k].name}`);
    assert.strictEqual(got[k].amount, w[1], `${msg} ${w[0]} 금액: ${got[k].amount}`);
    assert.strictEqual(got[k].rate, w[2], `${msg} ${w[0]} 분배율: ${got[k].rate}`);
  });
};

// ① 월중(2026-08-11 실제): 금액에 단위 없음, 분배율 있음
same(_solItems(head + '1 SOL 코리아고배당 60 0.46 2 SOL 미국배당미국채혼합50 21 0.18 ※ 분배금 공시일 : 26년 8월 11일'),
  [['SOL 코리아고배당', 60, 0.46], ['SOL 미국배당미국채혼합50', 21, 0.18]], '월중');

// ② 월말(2026-08-26 실제): **금액에 '원'이 붙는다** — 이걸 못 넘겨 8월 말 회차가 통째로 사라졌었다
same(_solItems(head + '1 SOL CD금리&머니마켓액티브 160원 0.32 2 SOL 국제금커버드콜액티브 46원 0.33 '
  + '3 SOL 금융지주플러스고배당 86원 0.38 4 SOL 200타겟위클리커버드콜 168원 1.39 ※ 지급 예정일 9월 1일'),
  [['SOL CD금리&머니마켓액티브', 160, 0.32], ['SOL 국제금커버드콜액티브', 46, 0.33],
   ['SOL 금융지주플러스고배당', 86, 0.38], ['SOL 200타겟위클리커버드콜', 168, 1.39]], '월말(원 표기)');

// ③ 분배율에 % 가 붙는 형식 + 금액에 콤마
same(_solItems(head + '1 SOL 미국배당다우존스 1,050원 0.28% ※'),
  [['SOL 미국배당다우존스', 1050, 0.28]], '% + 콤마');

// ④ 분배율을 아직 안 채운 회차 — 금액만 있어도 잡혀야 한다(나중 파싱에서 자동 갱신된다)
same(_solItems(head + '1 SOL 코리아고배당 60 2 SOL 미국S&P500미국채혼합50 18 ※'),
  [['SOL 코리아고배당', 60, null], ['SOL 미국S&P500미국채혼합50', 18, null]], '분배율 없음');

// ⑤ '분배금 내역' 자체가 없는 글이면 0건 — 엉뚱한 글에서 종목을 만들어내면 안 된다
assert.strictEqual(_solItems('SOL ETF 이벤트 안내 1 SOL 코리아고배당 60 0.46').length, 0, '표 없는 글에서 0건이어야 한다');

console.log('OK — SOL 표 파싱 5형식 통과 (월말 원 표기 포함)');
