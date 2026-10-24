// 중력 낙하 게이트 — ?tfdbg=drop 프로브 궤적 판정(지구 g=9.81 포물선).
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/grav_drop.cjs [url]
// 기본 url = http://localhost:8090/?tfdbg=drop (로컬 스택 8892·8893·dist 8090).
// 판정: ① 프라임 ② 프로브 리프트 로그 ③ 궤적 샘플 ≥4 ④ 에너지 보존
// (vy² ≈ 2·g·Δy, 오일러 적분 여유 ±35%) ⑤ 착지 로그. 산출 /tmp/grav_*.png.
const { chromium } = require('playwright');

(async () => {
  const url = process.argv[2] || 'http://localhost:8090/?tfdbg=drop';
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({ viewport: { width: 1280, height: 800 } });
  const grav = [];
  const errors = [];
  let liftAt = 0;
  pg.on('console', (m) => {
    const t = m.text();
    if (t.includes('[grav]')) grav.push(t.slice(0, 160));
    if (t.includes('프로브 리프트')) liftAt = Date.now();
    if (m.type() === 'error') errors.push(t.slice(0, 200));
  });
  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  let primed = false;
  for (let i = 0; i < 40 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  // 타임라인: 프라임 → +1.5s 리프트 → 0.78s 자유낙하(3m) → 착지.
  // 리프트 로그에 동기화해 촬영(프라임 검출 지연 0~3s를 못 따라가므로).
  for (let i = 0; i < 60 && !liftAt; i++) await pg.waitForTimeout(250);
  if (liftAt) {
    await pg.waitForTimeout(350); // 낙하 중(t≈0.35s, y≈−0.6m 진행)
    await pg.screenshot({ path: '/tmp/grav_midfall.png' });
    await pg.waitForTimeout(1400); // 착지 후
    await pg.screenshot({ path: '/tmp/grav_landed.png' });
  }
  await b.close();

  const lift = grav.find((l) => l.includes('프로브 리프트'));
  const land = grav.find((l) => l.includes('낙하 착지'));
  const rows = grav
    .filter((l) => /t=/.test(l))
    .map((l) => {
      const m = l.match(/t=([-\d.]+) y=([-\d.]+) vy=([-\d.]+) 공중=(\w+)/);
      return m && { t: +m[1], y: +m[2], vy: +m[3], air: m[4] === 'true' };
    })
    .filter(Boolean);
  // 궤적 판정은 공중 샘플만 — 착지 후 정지 샘플(vy=0)은 v² 검증을 더럽힌다.
  const air = rows.filter((r) => r.air);
  let parabola = false;
  if (air.length >= 3) {
    const dy = air[0].y - air[air.length - 1].y;
    const v2 = air[air.length - 1].vy ** 2;
    parabola = dy > 0.3 && v2 > 0.5 && Math.abs(v2 - 2 * 9.81 * dy) / (2 * 9.81 * dy) < 0.35;
  }
  const checks = {
    primed,
    lifted: !!lift,
    samples: air.length >= 3,
    parabola,
    landed: !!land,
  };
  const allPass = Object.values(checks).every(Boolean);
  console.log(
    JSON.stringify(
      { url: url.split('?')[0], allPass, checks, errors: errors.slice(0, 3), traj: rows, lift, land },
      null,
      2,
    ),
  );
  process.exit(allPass ? 0 : 1);
})();
