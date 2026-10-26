// 3D 클라이언트 hut(건축) 진단 — 라이브 프레임의 hut 자원 원문·Rust 스폰 로그·
// 현장 스크린샷을 한 번에 수집한다(2026-10-26 '집이 안 보여' 원인 분리용).
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_hut_probe.cjs [url]
// ① sink 랩으로 마지막 프레임 JSON 확보 → kind=hut 행·state 덤프
// ② Rust 콘솔 "[net]" 라인 수집 — 자원 실체 로그의 건물 수와 대조
// ③ hut에 가장 가까운 주민 selectAgent → 카메라 추적 → 스크린샷(/tmp/root3d_hut.png)
const { chromium } = require('playwright');
const fs = require('fs');

(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev';
  const PNG_OUT = process.env.PNG_OUT || '/tmp/root3d_hut.png';
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 1100, height: 760 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  const netLogs = [];
  const errors = [];
  pg.on('console', (m) => {
    const t = m.text();
    if (/\[net\]|panicked at|RustStdoutStderr/.test(t)) netLogs.push(t.slice(0, 400));
    if (m.type() === 'error') errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));

  // sink 랩 — Rust가 setSink(fn)으로 등록하기 전에 설치(모든 페이지 스크립트 선행)
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

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });

  // 프라임 대기(최대 180s) — 라이브 접속·지형 프라임 완료 신호
  let primed = false;
  for (let i = 0; i < 60 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }

  // 프레임 분석 — hut 행·종별 집계
  const frameInfo = await pg.evaluate(() => {
    const raw = window.__lastFrame;
    if (!raw) return { ok: false, reason: 'no frame' };
    let f;
    try { f = JSON.parse(raw); } catch (e) { return { ok: false, reason: 'parse: ' + e }; }
    const byKind = {};
    for (const r of f.resources) byKind[r.kind] = (byKind[r.kind] || 0) + 1;
    const huts = [];
    for (const r of f.resources) {
      if (r.kind !== 'hut') continue;
      let st = {};
      try { st = r.state ? JSON.parse(r.state) : {}; } catch (e) { /* pass */ }
      huts.push({ id: r.id, x: r.x, y: r.y, species: r.species, state: st });
    }
    const agents = (f.agents || []).filter((a) => a.kind === 'bot' || a.kind === 'user')
      .map((a) => ({ id: a.id, x: a.x, y: a.y }));
    return { ok: true, byKind, huts, agents, hour: f.hour };
  });

  // hut 현장 촬영 — 진짜 건축 행(state.stage 보유) 우선, 없으면 첫 hut.
  // 개굴(state {alive,nocturnal,aquatic,home} — stage 없음)과 건축을 구분.
  let camAgent = null;
  let camTarget = null;
  if (frameInfo.ok && frameInfo.huts.length > 0 && frameInfo.agents.length > 0) {
    const real = frameInfo.huts.filter((h) => h.state && h.state.stage);
    const h = (real[0] || frameInfo.huts[0]);
    camTarget = { id: h.id, x: h.x, y: h.y, stage: h.state.stage || null };
    let best = null;
    for (const a of frameInfo.agents) {
      const d = (a.x - h.x) ** 2 + (a.y - h.y) ** 2;
      if (!best || d < best.d) best = { a, d };
    }
    camAgent = best.a.id;
    await pg.evaluate((id) => {
      const tf = window.terrafolk;
      if (tf && tf.selectAgent) tf.selectAgent(id);
    }, camAgent);
    await pg.waitForTimeout(6000); // 카메라 팬 + 에셋 로딩 여유
  }
  await pg.screenshot({ path: PNG_OUT });
  await b.close();

  console.log(JSON.stringify({
    url, primed, hour: frameInfo.hour || null,
    frameOk: frameInfo.ok,
    reason: frameInfo.reason || null,
    byKind: frameInfo.byKind || null,
    hutCount: frameInfo.ok ? frameInfo.huts.length : -1,
    huts: frameInfo.ok ? frameInfo.huts : null,
    camAgent, camTarget,
    netLogs: netLogs.slice(0, 25),
    errSample: errors.slice(0, 8),
    png: PNG_OUT,
  }, null, 2));
})();
