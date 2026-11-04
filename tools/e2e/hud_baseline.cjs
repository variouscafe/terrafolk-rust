// HUD 텍스트 렌더 기준선 측정 — 상단 레일의 텍스트 클러스터 존재 판정.
// 사용: node tools/e2e/hud_baseline.cjs [url]   (기본 라이브 pages.dev)
// VW=390 등으로 뷰포트 폭 지정. 종료코드: 0=텍스트 확인, 1=소실/프라임 실패.
const { launchLive, waitPrimed, pngPixels } = require('./lib.cjs');
const fs = require('fs');

(async () => {
  const url = (process.argv[2] || 'https://terrafolk-3d.pages.dev') + '/?cb=' + Date.now();
  const out = process.env.PNG_OUT || '/tmp/hud_baseline.png';
  // 레일 스캔 높이 = bevy HUD_ROW_H(src/ui/hud.rs). 초과 시 레일 아래 월드
  // (밝은 물 등)가 클러스터로 잡혀 오탐(2026-11-04 리뷰 13).
  const RAIL_H = 38;
  const vw = +process.env.VW || 1280;
  const { b, pg } = await launchLive({ width: vw, height: 800 });
  try {
    await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
    const primed = await waitPrimed(pg, 180000);
    if (!primed) {
      console.log('FAIL: prime timeout(180s) — 스피너 화면은 판정 무의미');
      process.exitCode = 1;
      return;
    }
    await pg.waitForTimeout(2000);
    fs.writeFileSync(out, await pg.screenshot());
    const px = pngPixels(fs.readFileSync(out));
    const cols = {};
    const xmax = Math.min(px.w, vw > 600 ? 700 : vw - 5);
    for (let y = 0; y < RAIL_H; y++)
      for (let x = 0; x < xmax; x++) {
        const i = (y * px.w + x) * px.ch;
        const lum = 0.299 * px.data[i] + 0.587 * px.data[i + 1] + 0.114 * px.data[i + 2];
        if (lum > 150) cols[x] = (cols[x] || 0) + 1;
      }
    const xs = Object.keys(cols).map(Number).sort((a, b) => a - b);
    // 우측 정렬 칩 배치에선 클러스터가 통짜로 병합되므로 개수가 아닌
    // 밝은 열 총량으로 판정(PR #150 교체 — 회귀 오탐 방지).
    const present = xs.length > 50;
    console.log(`url=${url} vw=${vw}`);
    console.log(`bright cols=${xs.length} → HUD TEXT: ${present ? 'PRESENT' : 'MISSING'}`);
    if (!present) process.exitCode = 1;
  } finally {
    await b.close();
  }
})();
