// 전체 콘솔 수집 프로브 — wasm 오류·warn 패닉 라인 확인용.
// 사용: node tools/e2e/console_all.cjs [url]
const { launchLive, waitPrimed } = require('./lib.cjs');
(async () => {
  const url = process.argv[2] || 'http://221.143.21.29:5101';
  const { b, pg } = await launchLive({ width: 1280, height: 800 });
  const lines = [];
  pg.on('console', (m) => lines.push(`[${m.type()}] ${m.text().slice(0, 220)}`));
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg, 180000);
  await pg.waitForTimeout(5000);
  const uniq = [...new Set(lines)];
  console.log(uniq.join('\n') || '(no console)');
  await b.close();
})();
