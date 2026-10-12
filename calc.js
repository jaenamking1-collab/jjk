// =====================================================================
//  계산 한 곳 — portfolio.html(PC)과 m.html(폰)이 같이 쓴다 (2026-10-12).
//  서버는 평가금액·손익을 주지 않는다. 브라우저가 여기서 계산한다.
//  예전엔 두 파일이 같은 식을 따로 갖고 있어, 한쪽만 고치면 숫자가 조용히 갈렸다.
//  ⛔ 여기를 고치면 `node tools/calc_test.js` 를 돌린다(Actions calc-test 도 돈다).
//  공개 분배금공지(dist_notice.html)는 이 파일을 쓰지 않는다 — .assetsignore 로 공개 주소에서도 뺐다.
// =====================================================================
const Calc = (() => {
  const toKrw = (v, cur, rate) => cur === 'USD' ? v * rate : v;
  const ALLM = [1,2,3,4,5,6,7,8,9,10,11,12];

  // 받은 분배금 → 원화. 저장된 '그때 환율'(dividends 시트 rate 칸)이 있으면 그걸 쓴다.
  // 없으면 지금 환율로 떨어진다(rate 칸이 생기기 전의 옛 기록). 지난 달 합계가 오늘 환율에
  // 따라 매일 달라지던 것을 막는 것이 목적이다.
  // ⚠️ '앞으로 받을 예상'(투영)에는 쓰지 않는다 — 그건 오늘 환율이 맞다.
  function divKrw(amt, cur, rowRate, er) {
    const a = parseFloat(amt) || 0;
    return cur === 'USD' ? a * (parseFloat(rowRate) || er) : a;
  }

  // 종목 하나. 현재가가 없으면 매수평균가로 대신 계산한다(=손익 0, isLive=false).
  function holding(h, priceMap, rate) {
    const qty = parseFloat(h.quantity) || 0;
    const avg = parseFloat(h.avg_price) || 0;
    const live = priceMap[(h.ticker || '').toString().trim().toUpperCase()];
    const price = live || avg;
    const invest = toKrw(avg * qty, h.currency, rate);
    const value  = toKrw(price * qty, h.currency, rate);
    return { qty, avg, price, isLive: !!live, invest, value, pl: value - invest };
  }

  // 계좌 보유원금·평가금액. 국내(원화)와 해외(USD)를 **나눠서**도 돌려준다.
  // 왜: 증권사 앱은 '국내 잔고'와 '해외 잔고'를 다른 화면에 둔다. 합쳐서만 보여주니
  // 키움 화면과 우리 계좌 행이 안 맞아 보여 사용자가 틀린 줄 알았다(2026-09-21).
  // ⛔ 시세 없는 종목은 '본전'으로 보이므로 이름을 모아 화면에 밝힌다(noPrice).
  function investValue(hs, priceMap, rate) {
    let invest = 0, value = 0, investDom = 0, valueDom = 0, investOvs = 0, valueOvs = 0;
    const noPrice = [], noPriceDom = [], noPriceOvs = [];
    (hs || []).forEach(x => {
      const c = holding(x, priceMap, rate);
      invest += c.invest; value += c.value;
      if (!c.isLive && c.qty > 0) {
        const nm = x.name || x.ticker || '(이름없음)';
        noPrice.push(nm); (x.currency === 'USD' ? noPriceOvs : noPriceDom).push(nm);
      }
      if (x.currency === 'USD') { investOvs += c.invest; valueOvs += c.value; }
      else                      { investDom += c.invest; valueDom += c.value; }
    });
    return { invest, value, investDom, valueDom, investOvs, valueOvs, noPrice, noPriceDom, noPriceOvs };
  }

  // 분배금 기록 → { holding_id: { 연도: { 월: 금액 } } }, 그리고 행별 '그때 환율'
  function byHolding(dividends) {
    const byH = {}, divRate = {};
    (dividends || []).forEach(d => {
      const hid = d.holding_id, y = parseInt(d.year), m = parseInt(d.month);
      if (!hid || !y || !m) return;
      (byH[hid] = byH[hid] || {});
      (byH[hid][y] = byH[hid][y] || {});
      byH[hid][y][m] = parseFloat(d.amount) || 0;
      divRate[hid + '_' + y + '_' + m] = d.rate;
    });
    return { byH, divRate };
  }

  // 1회 예상액 = **최근 3회** 평균. 올해 전체 평균은 연중 추가 매수 전(수량 적던 때)의 작은 금액이
  // 끌어내려 월배당 48종목 합계가 월 ~75만원 낮게 잡혔다(2026-09-27 실측, 사용자 결정 A).
  // vals 는 월 오름차순(연도 넘어가면 연도 오름차순)이라 뒤에서 3개가 가장 최근이다.
  function perPayOf(byH, h, year) {
    const thisYear = (byH[h.id] && byH[h.id][year]) || {};
    let vals = Object.values(thisYear).filter(v => v > 0);
    if (!vals.length) vals = Object.values(byH[h.id] || {}).flatMap(o => Object.values(o)).filter(v => v > 0);
    const recent = vals.slice(-3);
    return { thisYear, vals, perPay: recent.length ? recent.reduce((a,b)=>a+b,0) / recent.length : 0 };
  }

  // 종목별 연간 분배금 투영(원화) — 계좌 표의 '환산분배율'·폰 종목 '환산 연분배금'이 쓴다.
  // 반환: { holding_id: 연 투영액(원) }. 대시보드(projectMonths)와 **같은 지급월 판정·같은 환산**이다.
  // 2026-10-12 이전엔 지급월을 '과거 전 연도 합집합'으로 세고 USD 를 달러 그대로 더해,
  // 밀린 반기 리츠는 대시보드와 연 예상이 달랐고 해외 줄 환산분배율이 1% 아래로 나왔다(사용자 결정: 둘 다 고침).
  function projectAnnual(holdings, dividends, year, rate) {
    return projectMonths(holdings, dividends, year, rate).perH;
  }

  // 월별 투영(대시보드·폰 대시보드). 반환 금액은 전부 원화.
  //   proj    확정+예상 12칸 (지금 보유 종목만 — 판 종목을 넣으면 과거 지급월이 앞으로도 들어오는 것처럼 부푼다)
  //   actual  입력 확정만 12칸 (판 종목의 올해 입력분도 더한다 — 실제로 받은 돈이다)
  //   who     그 달 투영에 뭐가 얼마나 들어갔나(막대 툴팁)
  //   runInfo 보유 종목별 최근 1회 분배금·연 지급 횟수(보유 기준 예상)
  function projectMonths(holdings, dividends, year, rate) {
    const { byH, divRate } = byHolding(dividends);
    // 지급월 판정. 예전엔 '과거 전 연도 지급월의 union'이라, 지급월이 해마다 밀리는
    // 반기 리츠(롯데리츠 3→4월, 코람코 2→3월)가 연 2회인데 3개월로 잡혀 특정 달이 부풀었다.
    // → 올해 실제 지급월 + 직전 연도 스케줄. 단 직전 월이 올해 월과 ±1개월 내면 '밀린 것'으로 보고 제외.
    const payMonthsOf = (h) => {
      const cyc = h.div_cycle || '';
      if (/월|주/.test(cyc)) return ALLM;
      const hist = byH[h.id] || {};
      const thisY = Object.keys(hist[year] || {}).map(Number).filter(Boolean);
      const priors = Object.keys(hist).map(Number).filter(y => y < year).sort((a,b)=>b-a);
      const ref = priors.length ? Object.keys(hist[priors[0]]).map(Number).filter(Boolean) : [];
      const merged = [...thisY];
      // ±1개월 드리프트만 병합. 2개월 이상 밀리는 경우는 드물어 무시.
      ref.forEach(m => { if (!thisY.some(a => Math.abs(a - m) <= 1)) merged.push(m); });
      const ms = [...new Set(merged)].sort((a,b)=>a-b);
      if (ms.length) return ms;
      if (/분기/.test(cyc)) return [3,6,9,12];
      if (/반기/.test(cyc)) return [6,12];
      return [];
    };
    const proj = Array(12).fill(0), actual = Array(12).fill(0);
    const who = Array.from({length:12}, () => []);
    const runInfo = [], perH = {};
    (holdings || []).forEach(h => {
      const cur = h.currency;
      perH[h.id] = 0;   // 판 종목은 앞으로 안 나온다
      const held = (parseFloat(h.quantity) || 0) > 0;
      const { thisYear, vals, perPay } = perPayOf(byH, h, year);
      if (held) runInfo.push({ h, perPayK: toKrw(perPay, cur, rate), lastK: vals.length ? toKrw(vals[vals.length - 1], cur, rate) : 0, n: payMonthsOf(h).length });
      (held ? payMonthsOf(h) : []).forEach(m => {
        const act = thisYear[m] || 0;
        const amtK = toKrw(act > 0 ? act : perPay, cur, rate);
        proj[m-1] += amtK; perH[h.id] += amtK;
        who[m-1].push({ name: h.name || h.ticker, amt: amtK, est: !(act > 0) });
      });
      // 확정(입력)은 지급월 제한 없이 실제 입력된 모든 월을 합산 → 앱 월합계와 일치
      Object.entries(thisYear).forEach(([mm, a]) => {
        const v = parseFloat(a) || 0;
        if (v > 0) actual[parseInt(mm)-1] += divKrw(v, cur, divRate[h.id + '_' + year + '_' + mm], rate);
      });
    });
    return { proj, actual, who, runInfo, byH, divRate, perH, annual: proj.reduce((a,b)=>a+b,0) };
  }

  // 지금 보유 기준 월 예상(대시보드 목표 카드 아래 한 줄). runInfo 는 projectMonths 의 것.
  //   월배당은 최근 공시 좌당 분배금 × 수량(일반 계좌는 종목 원천징수율을 뺀 세후),
  //   나머지는 입력 기준 한 달 치(분기·반기는 12달로 나눔). base = 입력 기준 한 달 치 합.
  //   perUnit: { 티커: 좌당 분배금 }, accClass(acc) → '일반'|'ISA'|…, taxOf(name) → 세율
  function goalRun(runInfo, perUnit, accounts, accClass, taxOf) {
    let run = 0, base = 0, fromNotice = 0;
    const noData = [];
    (runInfo || []).forEach(({ h, perPayK, lastK, n }) => {
      const qty = parseFloat(h.quantity) || 0;
      let t = String(h.ticker || '').trim().toUpperCase();
      if (h.currency !== 'USD' && /^\d{1,5}$/.test(t)) t = t.padStart(6, '0');
      const monthly = /월|주/.test(h.div_cycle || '');
      const acc = (accounts || []).find(a => a.id === h.account_id);
      const net = acc && accClass(acc) === '일반' ? 1 - taxOf(h.name) : 1;
      const b = monthly ? lastK : (perPayK > 0 && n > 0 ? perPayK * n / 12 : 0);
      base += b;
      if (monthly && perUnit[t]) { run += qty * perUnit[t] * net; fromNotice++; }
      else if (b > 0) run += b;
      else noData.push((h.name || h.ticker) + (acc ? ' (' + acc.name + ')' : ''));
    });
    return { run, base, fromNotice, noData };
  }

  return { divKrw, holding, investValue, projectAnnual, projectMonths, goalRun };
})();
if (typeof module !== 'undefined') module.exports = Calc;   // node tools/calc_test.js 용
