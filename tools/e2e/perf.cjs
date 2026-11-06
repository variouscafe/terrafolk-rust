// 모바일 웹뷰 성능 측정 — ?tfdbg=perf 로그 + rAF 프레임 간격, CPU 4x 스로틀(CDP), 세로 412×732 @2.625.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/perf.cjs <tag> [extraFlags] [runs=2] [url_base]
// 장면(각 15s): default(기본 줌) → close(근접 줌) → globe(최대 줌아웃). 결과 /tmp/perf_<tag>.json + 요약 표.
const { chromium } = require('playwright');
const fs = require('fs');
const { waitPrimed } = require('./lib.cjs');

const pct = (a, p) => { if (!a.length) return 0; const s = a.slice().sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(s.length * p))]; };
const mean = (a) => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : 0);

// 교차 A/B 모드: node perf.cjs ab <tag> <flagsA> <flagsB> [pairs=5] [url_base]  (예: "globe" "globe,no_globe_cull")
// 최대 줌아웃 한 장면만 ABAB… 로 번갈아 pairs쌍 — 각 15s(앞 3s 버림). 결과 /tmp/perf_ab_<tag>.json
async function ab() {
  const [, , , tag, fa, fb, np, ub] = process.argv;
  const pairs = +(np || 5);
  const base = ub || process.env.TF_URL || 'https://terrafolk-3d.pages.dev/';
  const one = async (label, fl, i) => {
    const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
    const ctx = await b.newContext({ viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625,
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36' });
    const pg = await ctx.newPage();
    const logs = [];
    pg.on('console', (m) => {
      const t = m.text(); const j = t.indexOf('[perf] ');
      if (j < 0) return;
      let s = t.slice(j + 7); const k = s.indexOf('}'); if (k > 0) s = s.slice(0, k + 1);
      try { logs.push(JSON.parse(s.trim())); } catch (e) { /* skip */ }
    });
    await pg.goto(`${base}?tfdbg=perf,${fl}&cb=${Date.now()}`, { waitUntil: 'load', timeout: 60000 });
    await waitPrimed(pg);
    await pg.waitForTimeout(8000);
    const cdp = await ctx.newCDPSession(pg);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
    await pg.evaluate(() => { window.__ft = []; let l = performance.now(); const f = (t) => { window.__ft.push(t - l); l = t; requestAnimationFrame(f); }; requestAnimationFrame(f); });
    await pg.mouse.move(206, 366);
    for (let k = 0; k < 40; k++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(100); }
    await pg.waitForTimeout(3000);
    await pg.evaluate(() => { window.__ft.length = 0; });
    logs.length = 0;
    await pg.waitForTimeout(15000);
    const ft = await pg.evaluate(() => window.__ft.slice());
    const vis = mean(logs.map((x) => x.vis_mesh ?? 0));
    await pg.screenshot({ path: `/tmp/perf_ab_${tag}_${label}${i}.png` });
    await b.close();
    const sec = {}; for (const k of ['pre', 'update', 'post_pre', 'warp', 'post_rest', 'main', 'props', 'gstat']) sec[k] = +mean(logs.map((x) => x[k] ?? 0)).toFixed(2);
    return { label, i, p50: +pct(ft, 0.5).toFixed(1), p95: +pct(ft, 0.95).toFixed(1), vis_mesh: Math.round(vis), n: ft.length, sec };
  };
  const rows = [];
  for (let i = 0; i < pairs; i++) { rows.push(await one('A', fa, i)); rows.push(await one('B', fb, i)); }
  const med = (a) => pct(a, 0.5);
  const A = rows.filter((r) => r.label === 'A'), B = rows.filter((r) => r.label === 'B');
  console.log(`pair  A(${fa}) p50 p95 vis | B(${fb}) p50 p95 vis | dP50(B-A) dP95`);
  const d50 = [], d95 = [];
  for (let i = 0; i < pairs; i++) {
    d50.push(B[i].p50 - A[i].p50); d95.push(B[i].p95 - A[i].p95);
    console.log(i, A[i].p50, A[i].p95, A[i].vis_mesh, '|', B[i].p50, B[i].p95, B[i].vis_mesh, '|', d50[i].toFixed(1), d95[i].toFixed(1));
  }
  console.log('[perf] 구간 ms (pair label pre update post_pre warp post_rest main | props gstat)');
  for (let i = 0; i < pairs; i++) for (const r of [A[i], B[i]]) { const q = r.sec; console.log(i, r.label, q.pre, q.update, q.post_pre, q.warp, q.post_rest, q.main, '|', q.props, q.gstat); }
  const msA = (k) => med(A.map((r) => r.sec[k])), msB = (k) => med(B.map((r) => r.sec[k]));
  console.log('구간 중앙값 ms A→B:', ['pre', 'post_pre', 'warp', 'post_rest', 'main'].map((k) => `${k} ${msA(k)}→${msB(k)}`).join(' · '));
  const neg = d50.filter((x) => x < 0).length;
  const mA = med(A.map((r) => r.p50)), mB = med(B.map((r) => r.p50));
  console.log(`median p50 A=${mA} B=${mB} improvement(B vs A)=${(((mA - mB) / mA) * 100).toFixed(1)}% ; B<A in ${neg}/${pairs} pairs ; median p95 A=${med(A.map((r) => r.p95))} B=${med(B.map((r) => r.p95))}`);
  fs.writeFileSync(`/tmp/perf_ab_${tag}.json`, JSON.stringify({ tag, fa, fb, rows }, null, 1));
}

(async () => {
  if (process.argv[2] === 'ab') return ab();
  const tag = process.argv[2] || 'base';
  const extra = process.argv[3] || '';
  const runs = +(process.argv[4] || 2);
  const base = process.argv[5] || process.env.TF_URL || 'https://terrafolk-3d.pages.dev/';
  const flags = ['perf', extra].filter(Boolean).join(',');
  const out = { tag, extra, runs: [] };
  for (let r = 0; r < runs; r++) {
    const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
    const ctx = await b.newContext({ viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625,
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36' });
    const pg = await ctx.newPage();
    const logs = [];
    let scene = 'boot';
    pg.on('console', (m) => {
      const t = m.text(); const i = t.indexOf('[perf] ');
      if (i < 0) return;
      let s = t.slice(i + 7); const k = s.indexOf('}'); if (k > 0) s = s.slice(0, k + 1);
      try { const o = JSON.parse(s.trim()); o.scene = scene; logs.push(o); } catch (e) { /* skip */ }
    });
    await pg.goto(`${base}?tfdbg=${flags}&cb=${Date.now()}`, { waitUntil: 'load', timeout: 60000 });
    await waitPrimed(pg);
    await pg.waitForTimeout(8000);
    const cdp = await ctx.newCDPSession(pg);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 });
    await pg.evaluate(() => { window.__ft = []; let l = performance.now(); const f = (t) => { window.__ft.push(t - l); l = t; requestAnimationFrame(f); }; requestAnimationFrame(f); });
    await pg.mouse.move(206, 366);
    const run = { run: r, scenes: {} };
    const measure = async (name) => {
      scene = name;
      await pg.waitForTimeout(3000);
      await pg.evaluate(() => { window.__ft.length = 0; });
      logs.length = 0; // 정착 구간 로그 제거
      await pg.waitForTimeout(15000);
      const ft = await pg.evaluate(() => window.__ft.slice());
      const L = logs.filter((x) => x.scene === name);
      const keys = ['fps', 'p50', 'p95', 'ents', 'vis_mesh', 'skins', 'ink', 'pre', 'update', 'post_pre', 'warp', 'post_rest', 'main', 'rest'];
      const avg = {}; for (const k of keys) avg[k] = +mean(L.map((x) => x[k] ?? 0)).toFixed(2);
      run.scenes[name] = { raf_n: ft.length, raf_fps: +(ft.length / 15).toFixed(1), raf_p50: +pct(ft, 0.5).toFixed(1), raf_p95: +pct(ft, 0.95).toFixed(1), perf_rows: L.length, perf: avg };
      await pg.screenshot({ path: `/tmp/perf_${tag}_r${r}_${name}.png` });
    };
    await measure('default');
    for (let i = 0; i < 12; i++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(100); }
    await measure('close');
    for (let i = 0; i < 40; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(100); }
    await measure('globe');
    out.runs.push(run);
    await b.close();
  }
  fs.writeFileSync(`/tmp/perf_${tag}.json`, JSON.stringify(out, null, 1));
  console.log('scene run raf_fps raf_p50 raf_p95 | perf fps p50 p95 ents vis skins ink | pre upd post_pre warp post_rest main rest');
  for (const run of out.runs) for (const [n, s] of Object.entries(run.scenes)) {
    const p = s.perf;
    console.log(n.padEnd(8), run.run, s.raf_fps, s.raf_p50, s.raf_p95, '|', p.fps, p.p50, p.p95, p.ents, p.vis_mesh, p.skins, p.ink, '|', p.pre, p.update, p.post_pre, p.warp, p.post_rest, p.main, p.rest);
  }
})();
