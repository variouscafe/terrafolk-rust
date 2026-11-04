// ?tfdbg=uidbg 부팅 → 콘솔 UIDBG 라인 수집(hud.rs uidbg_dump — t>8s 덤프).
// 사용: node tools/e2e/menu_uidbg.cjs [url]   (VW=폭)
// 덤프는 t≥8s에 1회 발화하므로 프라임 후 최대 15s까지 라인 도착을 기다린다
// (빠른 오리진에서 4s 수집으로 공허 종료하던 것 방지, 2026-11-04 리뷰 14).
const { launchLive, waitPrimed } = require('./lib.cjs');
(async () => {
  const url = (process.argv[2] || 'http://221.143.21.29:5101') + '/?tfdbg=uidbg';
  const { b, pg } = await launchLive({ width: +process.env.VW || 1280, height: 800 });
  const lines = [];
  pg.on('console', (m) => { const t = m.text(); if (t.includes('UIDBG')) lines.push(t); });
  try {
    await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
    const primed = await waitPrimed(pg, 180000);
    if (!primed) { console.log('FAIL: prime timeout'); process.exitCode = 1; return; }
    for (let i = 0; i < 15 && !lines.length; i++) await pg.waitForTimeout(1000);
    console.log(lines.sort().join('\n') || '(no UIDBG lines — 덤프 미발화)');
    if (!lines.length) process.exitCode = 1;
  } finally {
    await b.close();
  }
})();
