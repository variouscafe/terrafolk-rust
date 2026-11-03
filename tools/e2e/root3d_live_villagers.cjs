// 3D 클라이언트 라이브 배포 검증 — 주민 렌더·문명 점수(정착 실적) 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_live_villagers.cjs [url]
// 기본 url = https://terrafolk-3d.pages.dev. 개발 터널 검증은
//   http://221.143.21.29:5101 (tf-api CORS 등록 오리진 — 127.0.0.1:5101은 미등록).
// 판정: (1) 부팅 프라임(body.tf-loading 해제) (2) 캔버스 휘도 분산(스피너 아님)
//       (3) 오버레이 앵커 ≥1(라이브 주민 동기) (4) ☰→문명 팝업 오픈 + 요약
//       텍스트 판독("발명 N · 문명 점수 M" — 정착 실적 미반영이면 구식
//       최대치 N+12를 못 넘는다, 0.8.52+는 넘는다) (5) Rust panic 0.
//       주민 몸체 픽셀 판정은 하지 않는다 — 라벨 앵커+스크린샷 육안으로
//       보완할 것(스크린샷 /tmp/verify_live_world.png·_civ.png 저장).
// 유래: 2026-11-03 롤백 라인(0.8.52) 재배포 — 0.8.6x 무주민 사고 재발 방지.
const fs = require('fs');
const { pngPixels, boxStats, launchLive, waitPrimed } = require('./lib.cjs');

(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev';
  const PNG_WORLD = process.env.PNG_WORLD || '/tmp/verify_live_world.png';
  const PNG_CIV = process.env.PNG_CIV || '/tmp/verify_live_civ.png';
  const { b, pg, errors } = await launchLive();

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  const primed = await waitPrimed(pg);
  await pg.waitForTimeout(4000); // 오버레이·주민 정착 여유

  fs.writeFileSync(PNG_WORLD, await pg.screenshot());
  const px = pngPixels(fs.readFileSync(PNG_WORLD));
  const luma = boxStats(px, [
    Math.floor(px.w * 0.2), Math.floor(px.h * 0.35),
    Math.floor(px.w * 0.8), Math.floor(px.h * 0.85),
  ]);

  const anchors = await pg.evaluate(() => document.querySelectorAll('.tf-wa').length);
  const version = await pg.evaluate(() => {
    const el = document.getElementById('tf-version');
    return el ? el.textContent : '';
  });

  // 문명 팝업 — ☰ → 문명(inventions)
  let civText = '';
  try {
    await pg.click('.tf-menu-btn', { timeout: 5000 });
    await pg.waitForSelector('.tf-menu', { timeout: 5000 });
    await pg.click('.tf-menu-item[data-key="inventions"]', { timeout: 5000 });
    await pg.waitForTimeout(1200);
    civText = await pg.evaluate(() => {
      const el = document.querySelector('.tf-civ-summary-head');
      return el ? el.textContent.replace(/\s+/g, ' ').trim() : '';
    });
    fs.writeFileSync(PNG_CIV, await pg.screenshot());
  } catch (e) {
    errors.push('civ popup: ' + String(e).slice(0, 150));
  }
  await b.close();

  // 정착 실적 판정 — "발명 N · 문명 점수 M"에서 M > N+12(구식 최대 = N+6그룹×2)
  const m = /발명\s*(\d+)\s*·\s*문명 점수\s*(\d+)/.exec(civText || '');
  const invN = m ? Number(m[1]) : -1;
  const score = m ? Number(m[2]) : -1;
  const settleApplied = invN > 0 && score > invN + 12;

  const panic = errors.filter((e) => /panicked at/.test(e));
  const verdict = primed && luma && luma.std > 8 && anchors > 0 && civText && settleApplied && panic.length === 0;
  console.log(JSON.stringify({
    url, version, uiPrimed: primed,
    paintedStd: luma ? +luma.std.toFixed(1) : null,
    overlayAnchors: anchors, civSummary: civText,
    settleApplied, panicCount: panic.length, errSample: errors.slice(0, 4),
  }, null, 2));
  process.exit(verdict ? 0 : 1);
})();
