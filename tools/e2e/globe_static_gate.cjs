// G1/G2 게이트(globe_static): on/off 각 1회 — 기본줌→줌아웃 중간(애니메이션)→최대 줌아웃→기본줌 복귀 스크린샷 + 픽셀 비교.
// 사용: NODE_PATH=.../node_modules node tools/e2e/globe_static_gate.cjs <tag> [flags=globe,perf] [url_base]
// 출력 /tmp/gstat_<tag>_{on,off}_{default0,mid,globe,default1}.png, /tmp/gstat_<tag>.json (on/off 간 PNG 차이 픽셀 수)
const { chromium } = require('playwright');
const fs = require('fs');
const { waitPrimed, pngPixels } = require('./lib.cjs');
(async () => {
  const tag = process.argv[2] || 't';
  const fl0 = process.argv[3] || 'globe,perf';
  const base = process.argv[4] || 'https://terrafolk-3d.pages.dev/';
  const files = {}; const info = {};
  for (const [name, fl] of [['off', fl0 + ',no_globe_static'], ['on', fl0]]) {
    const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
    const ctx = await b.newContext({ viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625 });
    const pg = await ctx.newPage();
    let last = null; const chk = [];
    pg.on('console', (m) => { const t = m.text(); if (t.includes('[gstatchk]')) chk.push(t.slice(t.indexOf('[gstatchk]'))); const i = t.indexOf('[perf] '); if (i >= 0) { let s = t.slice(i + 7); const k = s.indexOf('}'); if (k > 0) s = s.slice(0, k + 1); try { last = JSON.parse(s.trim()); } catch (e) {} } });
    await pg.goto(`${base}?tfdbg=${fl}&cb=${Date.now()}`, { waitUntil: 'load', timeout: 60000 });
    await waitPrimed(pg);
    await pg.waitForTimeout(8000);
    await pg.mouse.move(206, 450);
    const shot = async (n) => { const p = `/tmp/gstat_${tag}_${name}_${n}.png`; await pg.screenshot({ path: p }); files[`${name}_${n}`] = p; };
    await shot('default0');
    // 중간 줌(애니메이션 도중): 휠 몇 틱 직후 즉시 캡처
    // 줌 5~6틱마다 캡처 — 곡률 램프(4.55~6.0) 중간 프레임 확보(mid1~3)
    for (let m = 1; m <= 3; m++) { for (let k = 0; k < 5; k++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(40); } await shot('mid' + m); }
    for (let k = 0; k < 10; k++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(60); }
    for (let k = 0; k < 30; k++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(100); }
    await pg.waitForTimeout(4000);
    await shot('globe');
    info[name + '_perf'] = last && { props: last.props, gstat: last.gstat, vis_mesh: last.vis_mesh };
    for (let k = 0; k < 60; k++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(100); }
    await pg.waitForTimeout(4000);
    await shot('default1');
    info[name + '_chk'] = chk.slice(-6);
    await b.close();
  }
  const diff = (a, b) => { const A = pngPixels(fs.readFileSync(a)), B = pngPixels(fs.readFileSync(b)); const x = A.data || A.px || A, y = B.data || B.px || B; let n = 0, mx = 0; const ch = (A.ch || 4); for (let i = 0; i < Math.min(x.length, y.length); i += ch) { const d = Math.abs(x[i] - y[i]) + Math.abs(x[i + 1] - y[i + 1]) + Math.abs(x[i + 2] - y[i + 2]); if (d > 30) n++; if (d > mx) mx = d; } return { diff_px: n, max: mx }; };
  const res = { info };
  for (const n of ['default0', 'default1', 'globe']) res[`on_vs_off_${n}`] = diff(files[`on_${n}`], files[`off_${n}`]);
  res.on_default0_vs_default1 = diff(files.on_default0, files.on_default1);
  res.off_default0_vs_default1 = diff(files.off_default0, files.off_default1);
  fs.writeFileSync(`/tmp/gstat_${tag}.json`, JSON.stringify({ files, res }, null, 1));
  console.log(JSON.stringify(res, null, 1));
})();
