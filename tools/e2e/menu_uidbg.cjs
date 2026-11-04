// ?tfdbg=uidbg 부팅 → 콘솔 UIDBG 라인 수집(hud.rs uidbg_dump).
// 사용: node tools/e2e/menu_uidbg.cjs [url]
const { launchLive, waitPrimed } = require('./lib.cjs');
(async () => {
  const url = (process.argv[2] || 'http://221.143.21.29:5101') + '/?tfdbg=uidbg';
  const { b, pg } = await launchLive({ width: +process.env.VW || 1280, height: 800 });
  const lines = [];
  pg.on('console', (m) => { const t = m.text(); if (t.includes('UIDBG')) lines.push(t); });
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg, 180000);
  await pg.waitForTimeout(4000);
  console.log(lines.sort().join('\n') || '(no UIDBG lines)');
  await b.close();
})();
