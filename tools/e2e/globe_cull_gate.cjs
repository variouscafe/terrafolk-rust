// G1/G2 게이트(globe_cull): cull on/off 각 1회 — 기본줌→최대줌아웃→기본줌 복귀 스크린샷·가시 메시·[globe_cull] 단계 로그.
// 사용: NODE_PATH=.../node_modules node tools/e2e/globe_cull_gate.cjs <tag> [extraFlagsOn=globe] [url_base]
// 출력 /tmp/gcull_<tag>_{on,off}_{default0,globe,default1}.png, /tmp/gcull_<tag>.json
const { chromium } = require('playwright');
const fs = require('fs');
const { waitPrimed } = require('./lib.cjs');
const mean = (a) => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : 0);
(async () => {
  const tag = process.argv[2] || 't';
  const on = process.argv[3] || 'globe';
  const base = process.argv[4] || 'https://terrafolk-3d.pages.dev/';
  const res = {};
  for (const [name, fl] of [['on', on], ['off', on + ',no_globe_cull']]) {
    const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
    const ctx = await b.newContext({ viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625 });
    const pg = await ctx.newPage();
    let scene = 'boot'; const perf = []; const stages = [];
    pg.on('console', (m) => {
      const t = m.text();
      let i = t.indexOf('[perf] ');
      if (i >= 0) { let s = t.slice(i + 7); const k = s.indexOf('}'); if (k > 0) s = s.slice(0, k + 1); try { const o = JSON.parse(s.trim()); o.scene = scene; perf.push(o); } catch (e) {} }
      i = t.indexOf('[globe_cull]');
      if (i >= 0) stages.push({ scene, msg: t.slice(i, i + 80) });
    });
    await pg.goto(`${base}?tfdbg=perf,${fl}&cb=${Date.now()}`, { waitUntil: 'load', timeout: 60000 });
    await waitPrimed(pg);
    await pg.waitForTimeout(8000);
    await pg.mouse.move(206, 366);
    const shot = async (n, wait) => {
      scene = n; await pg.waitForTimeout(wait); perf.length = 0; await pg.waitForTimeout(5000);
      await pg.screenshot({ path: `/tmp/gcull_${tag}_${name}_${n}.png` });
      res[`${name}_${n}`] = { vis_mesh: Math.round(mean(perf.filter((x) => x.scene === n).map((x) => x.vis_mesh ?? 0))), ents: Math.round(mean(perf.filter((x) => x.scene === n).map((x) => x.ents ?? 0))) };
    };
    await shot('default0', 1000);
    for (let k = 0; k < 40; k++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(100); }
    scene = 'globe_settle'; await pg.waitForTimeout(3000);
    stages.length = 0; // 정착 후 3s 무변화 확인용
    await shot('globe', 0);
    res[`${name}_globe`].stage_logs_while_idle = stages.filter((s) => s.scene === 'globe').length;
    for (let k = 0; k < 60; k++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(100); }
    await shot('default1', 2000);
    res[`${name}_stages`] = stages.map((s) => s.msg);
    await b.close();
  }
  fs.writeFileSync(`/tmp/gcull_${tag}.json`, JSON.stringify(res, null, 1));
  console.log(JSON.stringify(res, null, 1));
})();
