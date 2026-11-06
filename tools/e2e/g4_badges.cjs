// G4: 글로브 최대 줌아웃(?tfdbg=globe)에서 상태 배지가 주민 위(지면 하늘 아님)인지 — onOverlay 앵커 좌표 + DOM 배지 rect + 주석 스크린샷.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/g4_badges.cjs <tag> → /tmp/g4_<tag>.png (앵커 십자), 콘솔에 앵커·배지 rect 요약
const { chromium } = require('playwright');
const { waitPrimed } = require('./lib.cjs');
(async () => {
  const tag = process.argv[2] || 'x';
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const pg = await (await b.newContext({ viewport: { width: 900, height: 700 } })).newPage();
  await pg.addInitScript(() => {
    const t = (window.terrafolk = window.terrafolk || {});
    let last = null; let inner = null;
    Object.defineProperty(t, 'onOverlay', { configurable: true, get() { return function (j) { last = j; window.__lastOv = j; return inner && inner(j); }; }, set(f) { inner = f; } });
  });
  await pg.goto('https://terrafolk-3d.pages.dev/?tfdbg=globe&cb=' + Date.now(), { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg); await pg.waitForTimeout(15000);
  const r = await pg.evaluate(() => {
    const ov = JSON.parse(window.__lastOv || '{"a":[]}');
    const badges = [...document.querySelectorAll('.tf-wbadge')].filter((e) => e.style.display !== 'none').map((e) => { const r = e.getBoundingClientRect(); return { x: Math.round(r.x + r.width / 2), y: Math.round(r.y + r.height / 2) }; });
    return { zoom: ov.z, anchors: (ov.a || []).map((a) => ({ id: a[0], x: Math.round(a[1]), y: Math.round(a[2]) })), badges };
  });
  console.log(JSON.stringify(r));
  await pg.evaluate((anc) => { for (const a of anc) { const d = document.createElement('div'); d.style.cssText = `position:fixed;left:${a.x - 8}px;top:${a.y - 1}px;width:16px;height:2px;background:#f0f;z-index:99999;pointer-events:none`; document.body.appendChild(d); const e = document.createElement('div'); e.style.cssText = `position:fixed;left:${a.x - 1}px;top:${a.y - 8}px;width:2px;height:16px;background:#f0f;z-index:99999;pointer-events:none`; document.body.appendChild(e); } }, r.anchors);
  await pg.screenshot({ path: `/tmp/g4_${tag}.png` });
  await b.close();
})();
