// 섬 맵 로컬 렌더 선검증(2026-10-26) — 재생성 전 새 지형을 로컬 dist에서
// 실제 렌더해 본다(프레임 주입). 원줌 3단(근/중/원격+글로브) 스크린샷.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/island_preview.cjs
//   FRAME_PATH=/tmp/island_frame.json (기본) · LOCAL_URL=http://127.0.0.1:5174/
const { chromium } = require('playwright');
const fs = require('fs');

const LOCAL = process.env.LOCAL_URL || 'http://127.0.0.1:5174/';
const FRAME_PATH = process.env.FRAME_PATH || '/tmp/island_frame.json';
const PNG_OUT = process.env.PNG_OUT || '/tmp/island_preview';

(async () => {
  const frameJson = fs.readFileSync(FRAME_PATH, 'utf8');
  const frame = JSON.parse(frameJson);
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
  pg.on('console', (m) => {
    const t = m.text();
    if (/panicked at|pageerror/.test(t)) errors.push(t.slice(0, 300));
    if (m.type() === 'error') errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
  await pg.bringToFront();

  await pg.addInitScript(() => {
    window.__inject = { sink: null, frame: null };
    const t = setInterval(function () {
      const tf = window.terrafolk;
      if (tf && tf.setSink && !tf.__wrapped) {
        tf.__wrapped = true;
        const orig = tf.setSink;
        tf.setSink = function (fn) {
          window.__inject.sink = fn;
          return orig.call(this, function (json) { return fn(json); });
        };
        clearInterval(t);
      }
    }, 25);
  });
  await pg.goto(LOCAL, { waitUntil: 'load', timeout: 30000 });
  await pg.waitForFunction(() => !!window.__inject && window.terrafolk && window.terrafolk.__wrapped, null, { timeout: 60000 });
  await pg.evaluate((f) => { window.__inject.frame = f; }, frameJson);
  await pg.evaluate(() => {
    setInterval(function () {
      if (window.__inject.sink && window.__inject.frame) {
        try { window.__inject.sink(window.__inject.frame); } catch (e) { /* pass */ }
      }
    }, 700);
  });

  let primed = false;
  for (let i = 0; i < 50 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  if (!primed) {
    await b.close();
    console.error('프라임 실패');
    process.exit(1);
  }
  // 마을 광장 에이전트 선택 — 카메라 고정(섬 중앙)
  const a0 = frame.agents.find((a) => a.kind === 'bot' || a.kind === 'user');
  if (a0) {
    await pg.evaluate((id) => {
      const tf = window.terrafolk;
      if (tf && tf.selectAgent) tf.selectAgent(id);
    }, a0.id);
  }
  // 스폰 예산 여유(지형 프라임 직후 자원 스폰 진행 중)
  await pg.waitForTimeout(45000);
  await pg.screenshot({ path: PNG_OUT + '_near.png' });

  // 원줌 2단 — 마우스 휠(+delta=줌아웃). 캔버스 중앙 고정.
  await pg.mouse.move(550, 380);
  for (let i = 0; i < 14; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(120); }
  await pg.waitForTimeout(2500);
  await pg.screenshot({ path: PNG_OUT + '_mid.png' });
  for (let i = 0; i < 14; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(120); }
  await pg.waitForTimeout(2500);
  await pg.screenshot({ path: PNG_OUT + '_far.png' });
  await b.close();
  console.log(JSON.stringify({ primed, agent: a0 && a0.id, pngs: [PNG_OUT + '_near.png', PNG_OUT + '_mid.png', PNG_OUT + '_far.png'], errSample: errors.slice(0, 5) }));
  process.exit(0);
})();
