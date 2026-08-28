// PLUS 공지 OCR 행 파싱 자체 점검. 실행: node tools/test_plus_ocr.js
// Code.gs 안의 실제 plusOcrItems 를 그대로 떼어 돌린다(복사본을 두면 원본과 어긋난다).
const fs = require('fs'), assert = require('assert'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '..', 'Code.gs'), 'utf8');
const i = src.indexOf('function plusOcrItems'), j = src.indexOf('\nfunction fetchDist_plus', i);
assert.ok(i >= 0 && j > i, 'plusOcrItems 를 못 찾음');
const plusOcrItems = new Function(src.slice(i, j) + '\nreturn plusOcrItems;')();

// 2026-08-26 월말 공지 원문(13행). 분배율 칸에 % 가 없다.
const rows = [
  ['161510','PLUS 고배당주',103,0.41], ['251600','PLUS 고배당주채권혼합',50,0.32],
  ['489030','PLUS 고배당주위클리커버드콜',98,1.48], ['0210E0','PLUS 200커버드콜액티브',147,1.97],
  ['0203D0','PLUS 200위클리커버드콜채권혼합',63,0.70], ['429740','PLUS K리츠',27,0.44],
  ['0153X0','PLUS 미국고배당주액티브',48,0.41], ['0057H0','PLUS 미국S&P500미국채혼합50액티브',12,0.10],
  ['0089B0','PLUS 미국나스닥100미국채혼합50',10,0.09], ['332610','PLUS 미국단기회사채(AAA~A)',378,0.30],
  ['332620','PLUS 미국장기우량회사채',309,0.35], ['464470','PLUS 미국채30년액티브',93,0.19],
  ['0128D0','PLUS 차이나항셍테크위클리타겟커버드콜',88,1.24],
];
const same = (got, want, msg) => {
  assert.strictEqual(got.length, want.length, msg + ': 건수 ' + got.length + ' ≠ ' + want.length);
  want.forEach((w, k) => {
    assert.strictEqual(got[k].ticker, w[0], msg + ' ' + k + '행 코드');
    assert.strictEqual(got[k].name, w[1], msg + ' ' + w[0] + ' 이름: ' + got[k].name);
    assert.strictEqual(got[k].amount, w[2], msg + ' ' + w[0] + ' 금액: ' + got[k].amount);
    assert.strictEqual(got[k].rate, w[3], msg + ' ' + w[0] + ' 분배율: ' + got[k].rate);
  });
};

// ① 행 단위로 읽힌 정상 OCR
const perRow = rows.map((r, n) => `${n+1} ${r[0]} ${r[1]} ● ${r[2]} ${r[3].toFixed(2)}`).join(' ');
same(plusOcrItems('1. ETF별 분배금 종목코드 ETF명 분배금(원) 분배율(%) ' + perRow, '월말'), rows, '정상 행');

// ② 실제로 났던 고장 — 4·5행이 한 덩어리(코드 코드 이름 이름 값 값)로 묶여 나온 형태.
//    고치기 전에는 0210E0 이 통째로 빠지고 0203D0 이 147원(4행 값)을 가져갔다.
const blocked = rows.map((r, n) => n === 3 ? `4 ${r[0]}` :
  n === 4 ? `5 ${r[0]} ${rows[3][1]} ● ${r[1]} ● ${rows[3][2]} ${rows[3][3].toFixed(2)} ${r[2]} ${r[3].toFixed(2)}` :
  `${n+1} ${r[0]} ${r[1]} ● ${r[2]} ${r[3].toFixed(2)}`).join(' ');
const got = plusOcrItems(blocked, '월말');
same(got, rows, '두 행 묶임');
assert.ok(!got.some(x => (x.name.match(/PLUS/g) || []).length > 1), '이름에 두 종목이 붙으면 안 된다');

// ③ 월중 형식(분배율에 % 가 붙는다)
same(plusOcrItems('1 0018C0 PLUS 고배당주위클리고정커버드콜 ● 136 1.25% 2 495040 PLUS 코리아밸류업 ● 45 0.15%', '월중'),
  [['0018C0','PLUS 고배당주위클리고정커버드콜',136,1.25], ['495040','PLUS 코리아밸류업',45,0.15]], '월중 %');

// ④ 값 없는 코드가 쌍보다 많이 밀리면 가장 오래된 것을 버린다(엉뚱한 코드에 값이 붙지 않게).
const stray = plusOcrItems('161510 251600 489030 PLUS 고배당주위클리커버드콜 ● 98 1.48', '월말');
assert.strictEqual(stray.length, 1, '쌍이 하나면 한 건만 나와야 한다');
assert.strictEqual(stray[0].ticker, '489030', '값은 가장 가까운 코드에 붙어야 한다: ' + stray[0].ticker);

console.log('OK — PLUS OCR 행 파싱 ' + (rows.length * 2 + 2 + 2) + '건 통과');
