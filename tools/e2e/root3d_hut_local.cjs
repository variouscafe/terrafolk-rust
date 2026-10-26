// 로컬 3D 빌드 hut 렌더 검증 — 라이브 프레임 원문을 로컬 wasm에 주입해
// 완공 집·공사 발판·개굴 스킵을 데이터 정합 그대로 검증한다(2026-10-26).
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_hut_local.cjs
// 페이즈 A: /tmp/live_frame.json 없으면 배포 사이트에서 원문 프레임 캡처 저장.
// 페이즈 B: http://127.0.0.1:8096 로컬 dist 열고 setSink로 잡은 Rust sink에
//           저장 프레임을 700ms 주기 주입(로컬은 접속 실패 — 실서버 간섭 없음).
//           프라임 후 첫 완공 hut 근처 주민 선택 → 카메라 이동 → 스크린샷.
const { chromium } = require('playwright');
const fs = require('fs');

const LIVE = 'https://terrafolk-3d.pages.dev';
const LOCAL = process.env.LOCAL_URL || 'http://127.0.0.1:8096/';
const FRAME_PATH = process.env.FRAME_PATH || '/tmp/live_frame.json';
const PNG_OUT = process.env.PNG_OUT || '/tmp/root3d_hut_local.png';

async function launch() {
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 1100, height: 760 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  return [b, pg];
}

(async () => {
  // ── 페이즈 A — 라이브 프레임 원문 캡처(최초 1회) ──
  if (!fs.existsSync(FRAME_PATH)) {
    const [b, pg] = await launch();
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
    const logs = [];
    pg.on('console', (m) => { if (/\[net\]|panic/.test(m.text())) logs.push(m.text().slice(0, 200)); });
    await pg.goto(LIVE, { waitUntil: 'load', timeout: 30000 });
    let primed = false;
    for (let i = 0; i < 60 && !primed; i++) {
      await pg.waitForTimeout(3000);
      primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
    }
    const raw = await pg.evaluate(() => window.__lastFrame || null);
    await b.close();
    if (!raw) { console.error('프레임 캡처 실패'); process.exit(1); }
    fs.writeFileSync(FRAME_PATH, raw);
    console.log('프레임 캡처 저장:', FRAME_PATH, raw.length, 'bytes');
  }

  // ── 페이즈 B — 로컬 dist에 프레임 주입 ──
  const frameJson = fs.readFileSync(FRAME_PATH, 'utf8');
  const [b, pg] = await launch();
  const errors = [];
  const netLogs = [];
  let sawGlb = false;
  pg.on('console', (m) => {
    const t = m.text();
    if (/panicked at|pageerror/.test(t)) errors.push(t.slice(0, 300));
    if (m.type() === 'error') errors.push(t.slice(0, 300));
    if (/\[net\]|\[hutdbg\]/.test(t)) netLogs.push(t.slice(0, 220));
    if (t.includes('[hutdbg] glb')) sawGlb = true;
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
  await pg.bringToFront(); // 미포커스 창 RAF 스로틀 방지 — 스폰 예산(64/프레임) 풀림
  // Rust sink 납치 — Rust가 setSink(fn)으로 등록하면 fn을 보관, 주입 시작
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
  // sink 확보 대기 후 주입 루프(페이지 컨텍스트 interval — 탐색 종료 후에도 유지)
  await pg.waitForFunction(() => !!window.__inject && window.terrafolk && window.terrafolk.__wrapped, null, { timeout: 60000 });
  await pg.evaluate((frame) => { window.__inject.frame = frame; }, frameJson);
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
  // 단계별 캡처 — 프라임 직후(스폰 폭풍 전) 렌더 건재 확인용
  await pg.screenshot({ path: PNG_OUT.replace('.png', '_p0.png') });
  // 5초 fps 측정 — 스톰 진행 중 프레임 루프 속도 추정
  const fps = await pg.evaluate(() => new Promise((res) => {
    let n = 0; const t0 = performance.now();
    const tick = () => { n += 1; if (performance.now() - t0 < 5000) requestAnimationFrame(tick); else res(n / 5); };
    requestAnimationFrame(tick);
  }));
  console.error('fps≈' + fps.toFixed(1));
  // hut 스폰 완료 대기 — 예산(64/프레임)이 5천+ 자원을 도는 시간 확보
  // (60fps 기준 ~90콜 ≈ 수 초. 로그 의존 없이 고정 대기).
  // rAF 생존 검사 병행 — 프레임 루프 멈춤(스로틀·프리즈)과 도달 지연을 구분.
  let rafAlive = true;
  for (let i = 0; i < 20; i++) {
    await pg.waitForTimeout(1500);
    rafAlive = await pg.evaluate(() => new Promise((res) => {
      const t = setTimeout(() => res(false), 1200);
      requestAnimationFrame(() => { clearTimeout(t); res(true); });
    }));
    if (!rafAlive) break;
  }

  // 첫 완공 hut 근처 주민 선택 → 카메라 이동
  const frame = JSON.parse(frameJson);
  const blds = frame.resources.filter((r) => {
    try { return r.kind === 'hut' && r.state && JSON.parse(r.state).stage === 'COMPLETED'; } catch (e) { return false; }
  });
  const dens = frame.resources.filter((r) => {
    try { const s = r.state && JSON.parse(r.state); return r.kind === 'hut' && s && !s.stage && ('alive' in s); } catch (e) { return false; }
  });
  let camAgent = null, camTarget = null;
  if (blds.length > 0) {
    const h = blds[0];
    camTarget = { id: h.id, x: h.x, y: h.y };
    // 결정론 카메라 — 첫 bot을 hut 발판 중심으로 이동시킨 프레임으로 재주입.
    // '가장 가까운 주민'은 실시간 좌표라 표적 이탈이 생겨 판정이 흐려진다.
    const a0 = frame.agents.find((a) => a.kind === 'bot' || a.kind === 'user');
    if (a0) {
      // 발판 남쪽 2타일 밖 — 집 안이 아니라 집을 '바라보는' 카메라
      a0.x = h.x + 1.5; a0.y = h.y + 3.5;
      camAgent = a0.id;
      const forced = JSON.stringify(frame);
      await pg.evaluate((f2) => { window.__inject.frame = f2; }, forced);
      await pg.waitForTimeout(1500); // 재주입 프레임 수용
      await pg.evaluate((id) => {
        const tf = window.terrafolk;
        if (tf && tf.selectAgent) tf.selectAgent(id);
      }, camAgent);
    }
  }
  await pg.waitForTimeout(75000); // 스폰 폭풍(5천 행 × 씬 인스턴스화) 종료 여유
  await pg.screenshot({ path: PNG_OUT.replace(/\.png$/, '_a.png') });
  await pg.waitForTimeout(4000);
  await pg.screenshot({ path: PNG_OUT });
  await b.close();

  console.log(JSON.stringify({
    local: LOCAL, primed, rafAlive, sawGlb, completedBlds: blds.length, dens: dens.length,
    camAgent, camTarget, png: PNG_OUT,
    netLogs: netLogs.slice(-60), errSample: errors.slice(0, 6),
  }, null, 2));
  process.exit(primed ? 0 : 1);
})();
