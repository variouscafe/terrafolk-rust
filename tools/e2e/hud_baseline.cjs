// HUD 텍스트 렌더 기준선 측정 — 상단 레일의 텍스트 클러스터 존재 판정.
// 사용: node tools/e2e/hud_baseline.cjs [url]   (기본 라이브 pages.dev)
const { launchLive, waitPrimed } = require('./lib.cjs');
const fs = require('fs');
const { pngPixels } = require('./lib.cjs');
(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev';
  const out = process.env.PNG_OUT || '/tmp/hud_baseline.png';
  const { b, pg } = await launchLive({ width: +process.env.VW || 1280, height: 800 });
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg, 180000);
  await pg.waitForTimeout(2000);
  fs.writeFileSync(out, await pg.screenshot());
  // 레일(y 0..44)에서 텍스트 클러스터 — x<700 구간 밝은 픽셀
  const shot = fs.readFileSync(out);
  const px = pngPixels(shot);
  const cols = {};
  for (let y = 0; y < 44; y++)
    for (let x = 0; x < Math.min(px.w, +process.env.VW || 700); x++) {
      const i = (y * px.w + x) * px.ch;
      const lum = 0.299 * px.data[i] + 0.587 * px.data[i + 1] + 0.114 * px.data[i + 2];
      if (lum > 150) cols[x] = (cols[x] || 0) + 1;
    }
  const xs = Object.keys(cols).map(Number).sort((a, b) => a - b);
  const clusters = [];
  if (xs.length) {
    let s = xs[0], p = xs[0];
    for (const x of xs) { if (x - p > 25) { clusters.push([s, p]); s = x; } p = x; }
    clusters.push([s, p]);
  }
  console.log(`url=${url}`);
  console.log(`bright cols=${xs.length} clusters=${JSON.stringify(clusters.slice(0, 8))}`);
  console.log(xs.length > 50 ? 'HUD TEXT: PRESENT' : 'HUD TEXT: MISSING');
  await b.close();
})();
