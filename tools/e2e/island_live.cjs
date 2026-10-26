// 라이브 섬 맵 검증(2026-10-26) — 배포 사이트(terrafolk-3d.pages.dev)에서
// 프라임 후 원줌 3단 스크린샷 + 프레임 맵 통계. 주입 없음(실서버 데이터 그대로).
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/island_live.cjs
const { chromium } = require('playwright');

const LIVE = process.env.LIVE_URL || 'https://terrafolk-3d.pages.dev';
const PNG_OUT = process.env.PNG_OUT || '/tmp/island_live';

(async () => {
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 1100, height: 760 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  const errors = [];
  let lastFrame = null;
  pg.on('console', (m) => {
    const t = m.text();
    if (/panicked at/.test(t)) errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
  await pg.bringToFront();
  await pg.addInitScript(() => {
    const t = setInterval(function () {
      const tf = window.terrafolk;
      if (tf && tf.setSink && !tf.__wrapped) {
        tf.__wrapped = true;
        const orig = tf.setSink;
        tf.setSink = function (fn) {
          return orig.call(this, function (json) {
            window.__lastFrame = json;
            return fn(json);
          });
        };
        clearInterval(t);
      }
    }, 30);
  });
  await pg.goto(LIVE, { waitUntil: 'load', timeout: 30000 });
  let primed = false;
  for (let i = 0; i < 60 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  if (!primed) { await b.close(); console.error('프라임 실패'); process.exit(1); }

  lastFrame = await pg.evaluate(() => window.__lastFrame || null);
  let stats = null;
  if (lastFrame) {
    const f = JSON.parse(lastFrame);
    if (f.map) {
      const t = JSON.parse(f.map);
      const d = t.layers[0].data, n = t.width;
      const water = sum(1)(d, (g) => g === 1);
      const borders = all(d, n);
      stats = {
        width: n, waterPct: +(100 * water / d.length).toFixed(1),
        bordersAllWater: borders,
        resources: f.resources.length, agents: f.agents.length,
      };
      function sum(k) { return (arr, fn) => arr.reduce((a, v) => a + (fn(v) ? 1 : 0), 0); }
      function all(arr, size) {
        for (const y of [0, size - 1]) for (let x = 0; x < size; x++) if (arr[y * size + x] !== 1) return false;
        for (const x of [0, size - 1]) for (let y = 0; y < size; y++) if (arr[y * size + x] !== 1) return false;
        return true;
      }
    }
  }
  // 스폰 예산 여유 후 주민 선택(카메라 = 마을)
  await pg.waitForTimeout(40000);
  const frame = lastFrame ? JSON.parse(lastFrame) : null;
  const a0 = frame && frame.agents.find((a) => a.kind === 'bot' || a.kind === 'user');
  if (a0) {
    await pg.evaluate((id) => {
      const tf = window.terrafolk;
      if (tf && tf.selectAgent) tf.selectAgent(id);
    }, a0.id);
  }
  await pg.screenshot({ path: PNG_OUT + '_near.png' });
  await pg.mouse.move(550, 380);
  for (let i = 0; i < 14; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(120); }
  await pg.waitForTimeout(2500);
  await pg.screenshot({ path: PNG_OUT + '_mid.png' });
  for (let i = 0; i < 14; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(120); }
  await pg.waitForTimeout(3000);
  await pg.screenshot({ path: PNG_OUT + '_far.png' });
  await b.close();
  console.log(JSON.stringify({ primed, stats, pngs: [PNG_OUT + '_near.png', PNG_OUT + '_mid.png', PNG_OUT + '_far.png'], errSample: errors.slice(0, 5) }));
  process.exit(0);
})();
