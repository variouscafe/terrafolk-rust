// 거대 메시 프로브 수집 — ?tfdbg=globe,bigmesh 세로 폰, 휠 줌아웃 2단계 후 40s 콘솔 수집 → /tmp/bigmesh.json
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/bigmesh.cjs [globe|flat] [url_base]
const { chromium } = require('playwright');
const fs = require('fs');
const { waitPrimed } = require('./lib.cjs');
(async () => {
  const mode = process.argv[2] || 'globe';
  const base = process.argv[3] || 'https://terrafolk-3d.pages.dev/';
  const url = `${base}?tfdbg=${mode === 'globe' ? 'globe,' : ''}bigmesh&cb=${Date.now()}`;
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625,
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36' });
  const pg = await ctx.newPage();
  const rows = [];
  pg.on('console', async (m) => {
    const t = m.text(); const i = t.indexOf('[bigmesh] ');
    if (i < 0) return;
    let s = t.slice(i + 10);
    const k = s.indexOf('%c'); if (k > 0) s = s.slice(0, k);
    try { rows.push(JSON.parse(s.trim())); } catch (e) { rows.push({ raw: s.slice(0, 40000) }); }
  });
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg);
  await pg.waitForTimeout(4000);
  await pg.mouse.move(206, 366);
  for (let i = 0; i < 2; i++) { await pg.mouse.wheel(0, 120); await pg.waitForTimeout(1500); }
  await pg.waitForTimeout(40000);
  await pg.screenshot({ path: `/tmp/bigmesh_${mode}.png` });
  await b.close();
  fs.writeFileSync(`/tmp/bigmesh${mode === 'globe' ? '' : '_' + mode}.json`, JSON.stringify(rows, null, 1));
  // A5 게이트: wear-* 조상 항목 extent > 3.0m 0건(flat·globe 공통). 프로브는 extent>8 항목만 로그하므로 0건이면 통과.
  const dec = (r) => { try { return JSON.parse(r.raw.replace(/\} color:.*$/, '}')); } catch (e) { return r; } };
  let wearBad = 0, wearRendered = 0;
  for (const r of rows) {
    const o = r.top ? r : dec(r);
    for (const t of o.top || []) {
      const isWear = t.ancestors.some((a) => a.startsWith('wear-'));
      if (isWear && t.extent > 3.0) { wearBad++; if (t.view_vis) wearRendered++; }
    }
  }
  console.log('wearBad(>3m, 프로브는 >8m만 로그)', wearBad, '그중 view_vis(실제 렌더 대상)', wearRendered, wearBad === 0 ? 'PASS' : 'FAIL');
  console.log(rows.length, 'rows');
})();
