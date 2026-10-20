// calc.js 검사 — `node tools/calc_test.js`. Actions(calc-test.yml)도 푸시마다 돌린다.
// ① 가짜 보유·분배 데이터(실제 금액 아님 — 저장소가 공개다)로 낸 숫자가 기록된 정답과 같은지
// ② PC·폰 화면이 둘 다 calc.js 를 불러 쓰고, 계산식을 다시 따로 갖고 있지 않은지
// 정답을 일부러 바꿨을 때만: node tools/calc_test.js --update
const fs = require('fs'), path = require('path');
const root = path.join(__dirname, '..');
const Calc = require(path.join(root, 'calc.js'));
const goldenPath = path.join(__dirname, 'calc_golden.json');

const RATE = 1400, YEAR = 2026;
const holdings = [
  { id:'h1', account_id:'a1', ticker:'KR1',  name:'월배당A', quantity:'100', avg_price:'10000', currency:'KRW', div_cycle:'월' },
  { id:'h2', account_id:'a1', ticker:'us1 ', name:'미국B',   quantity:'10',  avg_price:'20',    currency:'USD', div_cycle:'월' },
  { id:'h3', account_id:'a2', ticker:'KR3',  name:'반기리츠', quantity:'50',  avg_price:'5000',  currency:'KRW', div_cycle:'반기' },
  { id:'h4', account_id:'a2', ticker:'KR4',  name:'시세없음', quantity:'7',   avg_price:'3000',  currency:'KRW', div_cycle:'분기' },
  { id:'h5', account_id:'a2', ticker:'KR5',  name:'판종목',   quantity:'0',   avg_price:'8000',  currency:'KRW', div_cycle:'월' },
];
const priceMap = { KR1: 10500, US1: 22, KR3: 4800, KR5: 9000 };
const dv = (h, y, m, amount, rate) => ({ holding_id:h, year:y, month:m, amount:String(amount), rate, currency: h === 'h2' ? 'USD' : 'KRW' });
const dividends = [
  dv('h1',2026,1,700), dv('h1',2026,2,720), dv('h1',2026,3,750), dv('h1',2026,4,800),
  dv('h2',2026,1,1.5,'1350'), dv('h2',2026,2,1.6,''), dv('h2',2026,3,1.7),
  dv('h3',2025,3,1500), dv('h3',2025,9,1500), dv('h3',2026,4,1600),   // 3월→4월로 밀린 반기 리츠
  dv('h4',2025,3,200), dv('h4',2025,6,210),
  dv('h5',2026,1,500), dv('h5',2026,2,500),
];

const months = Calc.projectMonths(holdings, dividends, YEAR, RATE);
const result = {
  investValue: Calc.investValue(holdings, priceMap, RATE),
  holdings: holdings.map(h => Calc.holding(h, priceMap, RATE)),
  projectAnnual: Calc.projectAnnual(holdings, dividends, YEAR, RATE),
  projectMonths: { proj: months.proj, actual: months.actual, annual: months.annual,
                   runInfo: months.runInfo.map(x => [x.h.id, x.perPayK, x.lastK, x.n]) },
  divKrw: dividends.map(d => Calc.divKrw(d.amount, d.currency, d.rate, RATE)),
  goalRun: Calc.goalRun(months.runInfo, { KR1: 80 }, [{ id:'a1', name:'일반' }, { id:'a2', name:'ISA' }],
                        a => a.name, () => 0.154),
};
const round = o => JSON.parse(JSON.stringify(o, (k, v) => typeof v === 'number' ? Math.round(v * 1e6) / 1e6 : v));

let fail = 0;
const bad = msg => { console.log('❌ ' + msg); fail++; };
if (process.argv.includes('--update')) {
  fs.writeFileSync(goldenPath, JSON.stringify(round(result), null, 1) + '\n');
  console.log('정답 갱신: ' + goldenPath);
} else {
  const want = JSON.parse(fs.readFileSync(goldenPath, 'utf8')), got = round(result);
  Object.keys(want).forEach(k => { if (JSON.stringify(want[k]) !== JSON.stringify(got[k]))
    bad(`${k} 값이 바뀌었다\n   정답 ${JSON.stringify(want[k]).slice(0, 400)}\n   지금 ${JSON.stringify(got[k]).slice(0, 400)}`); });
}

// ①-2 달러 원금은 산 날 환율로 고정한 krw_cost 를 쓰고, 오늘 환율이 바뀌어도 그대로다(2026-10-20)
{
  const h = { quantity:'10', avg_price:'20', currency:'USD', krw_cost:'270000', ticker:'US1' };
  const a = Calc.holding(h, priceMap, 1400), b = Calc.holding(h, priceMap, 1500);
  if (a.invest !== 270000 || b.invest !== 270000) bad('krw_cost 가 있는데 원금이 오늘 환율로 바뀐다');
  if (Calc.costKrw({ quantity:'10', avg_price:'20', currency:'USD' }, 1400) !== 280000) bad('krw_cost 없는 달러 종목은 오늘 환율이어야 한다');
}

// ② 화면 파일 구조
['portfolio.html', 'm.html'].forEach(f => {
  const src = fs.readFileSync(path.join(root, f), 'utf8');
  if (!/<script src="calc\.js(\?v=\d+)?"><\/script>/.test(src)) bad(`${f} 가 calc.js 를 불러오지 않는다`);
  if (/payMonthsOf *=|function payMonthsOf|vals\.slice\(-3\)/.test(src)) bad(`${f} 에 투영 계산식이 다시 생겼다 — calc.js 로 옮길 것`);
});
const pub = fs.readFileSync(path.join(root, 'dist_notice.html'), 'utf8');
if (/calc\.js/.test(pub)) bad('dist_notice.html(공개 페이지)가 calc.js 를 부른다 — 공개 주소엔 이 파일이 없다');

console.log(fail ? `실패 ${fail}건` : '✅ 계산 검사 통과');
process.exit(fail ? 1 : 0);
