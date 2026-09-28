// 분배금 예측 A(최근 3회 금액 평균) vs B(최근 3회 1주당 × 지금 수량) 실데이터 비교.
// 실행: node tools/forecast_compare.js boot.json cost.json   (forecast-compare 워크플로가 부른다)
// m.html 의 실제 projectMonthly 를 떼어 원금월별을 넣은 것/뺀 것으로 두 번 돌린다.
// ⛔ jjk 는 공개 저장소라 Actions 로그도 공개다 — **합계만** 찍는다. 종목명·수량·계좌는 찍지 않는다.
const fs = require('fs'), path = require('path');
const src = fs.readFileSync(path.join(__dirname, '..', 'm.html'), 'utf8');
const grab = from => { const i = src.indexOf(from), j = src.indexOf('\n}\n', i); if (i < 0 || j < 0) throw new Error('못 찾음: ' + from); return src.slice(i, j + 2); };
const code = ['function toKrw(', 'function divKrwOf(', 'function perPayEstimate(', 'function costFor(', 'function projectMonthly('].map(grab).join('\n');
// 파싱 오류 메시지엔 원문 일부가 섞이므로 길이만 찍는다(공개 로그).
const load = f => { const s = fs.readFileSync(f, 'utf8'); try { return JSON.parse(s); } catch (e) { throw new Error(f + ' 가 JSON 이 아님 (' + s.length + '자)'); } };
const boot = load(process.argv[2]), cost = load(process.argv[3]);
if (!boot.holdings || !boot.divsAll) throw new Error('getBootstrap 응답이 이상하다: ' + Object.keys(boot).join(','));
if (cost.error || !Object.keys(cost).length) throw new Error('getCostBasis 응답이 비었다');

const Y = new Date().getFullYear();
const run = c => new Function('D', 'localStorage', code + '\nreturn projectMonthly;')(
  { rate: boot.rate || 1447, cost: c }, { getItem: () => null })(boot.holdings, boot.divsAll, Y);
const A = run(null), B = run(cost);

// 종목 몇 개가 B 로 계산됐는지(나머지는 수량 이력이 없어 A 로 폴백)
const perPay = new Function(grab('function perPayEstimate(') + '\nreturn perPayEstimate;')();
const byH = {};
boot.divsAll.forEach(d => { const y = +d.year, m = +d.month; if (!d.holding_id || !y || !m) return;
  ((byH[d.holding_id] = byH[d.holding_id] || {})[y] = byH[d.holding_id][y] || {})[m] = parseFloat(d.amount) || 0; });
let nB = 0, nA = 0, up = 0, down = 0;
boot.holdings.filter(h => (parseFloat(h.quantity) || 0) > 0 && byH[h.id]).forEach(h => {
  const a = perPay(h, byH[h.id], Y, null), b = perPay(h, byH[h.id], Y, cost);
  if (a === b) nA++; else { nB++; if (b > a) up++; else down++; }
});

const man = v => (v / 10000).toFixed(0).replace(/\B(?=(\d{3})+(?!\d))/g, ',') + '만';
const now = new Date().getMonth() + 1;
console.log(`${Y}년 예상 분배금 (KRW, 확정+예상) — A → B`);
for (let m = now; m <= 12; m++) console.log(`  ${m}월: ${man(A.proj[m-1])} → ${man(B.proj[m-1])}`);
console.log(`  월평균: ${man(A.annual / 12)} → ${man(B.annual / 12)}  (목표 1,000만)`);
console.log(`보유 종목 중 B 로 계산: ${nB}개 (올라감 ${up} · 내려감 ${down}) · 그대로: ${nA}개 (최근 3회 수량이 같았거나 수량 이력이 없어 A 와 같음)`);
