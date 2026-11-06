// G1 wearfit 수집·판정 — ?tfdbg=wearfit,wearglasses,wearbeard 기본 줌, N초 콘솔 수집 → /tmp/wearfit_<tag>.json
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/wearfit.cjs [tag] [secs] [url_base]
// 판정: violations 합 0 · hair_brow_wear_meshes 합 0 (주민 1명 이상 로그 필요).
const { chromium } = require('playwright');
const fs = require('fs');
const { waitPrimed } = require('./lib.cjs');
(async () => {
  const tag = process.argv[2] || 'x';
  const secs = +(process.argv[3] || 40);
  const base = process.argv[4] || 'https://terrafolk-3d.pages.dev/';
  const url = `${base}?tfdbg=wearfit,wearglasses,wearbeard${process.env.TF_EXTRA ? ',' + process.env.TF_EXTRA : ''}&cb=${Date.now()}`;
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ viewport: { width: 900, height: 700 } });
  const pg = await ctx.newPage();
  const rows = [];
  pg.on('console', (m) => {
    const t = m.text(); const i = t.indexOf('[wearfit] ');
    if (i < 0) return;
    let s = t.slice(i + 10); const k = s.indexOf('%c'); if (k > 0) s = s.slice(0, k);
    s = s.replace(/\} color:.*$/, '}');
    try { rows.push(JSON.parse(s.trim())); } catch (e) { rows.push({ raw: s.slice(0, 3000) }); }
  });
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg);
  await pg.waitForTimeout(secs * 1000);
  await b.close();
  fs.writeFileSync(`/tmp/wearfit_${tag}.json`, JSON.stringify(rows, null, 1));
  const ok = rows.filter((r) => r.head);
  const viol = ok.reduce((a, r) => a + r.violations, 0);
  const hair = ok.reduce((a, r) => a + r.hair_brow_wear_meshes, 0);
  console.log(`villagers=${ok.length} raw=${rows.length - ok.length} violations=${viol} hair_brow_wear_meshes=${hair}`, ok.length && !viol && !hair ? 'PASS' : 'FAIL');
})();
