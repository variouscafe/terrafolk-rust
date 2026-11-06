// 맵↔객체 감사 수집 — ?tfdbg=mapaudit 콘솔의 [mapaudit] JSON 라인을 프라임 후 20s 모아 저장.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/map_audit.cjs [url] [out.json]
const { launchLive, waitPrimed } = require('./lib.cjs');
const fs = require('fs');
(async () => {
  const url = process.argv[2] || `https://terrafolk-3d.pages.dev/?tfdbg=mapaudit&cb=${Date.now()}`;
  const out = process.argv[3] || '/tmp/map_audit.json';
  const { b, pg } = await launchLive({ width: 1280, height: 800 });
  const rows = [];
  pg.on('console', (m) => {
    const i = m.text().indexOf('[mapaudit] ');
    if (i < 0) return;
    const j = m.text().slice(i + 11).replace(/%c.*$/, '').trim();
    try { rows.push(JSON.parse(j)); } catch (e) { rows.push({ raw: j.slice(0, 300) }); }
  });
  try {
    await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
    await waitPrimed(pg, 180000);
    await pg.waitForTimeout(20000);
  } finally { await b.close(); }
  fs.writeFileSync(out, JSON.stringify(rows, null, 1));
  console.log(rows.length, 'rows'); console.log(JSON.stringify(rows[rows.length - 1] || null));
})();
