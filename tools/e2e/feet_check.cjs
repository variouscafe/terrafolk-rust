// 발 접지 검증 프로브(2026-10-23, 6차 '뒤꿈치 부유' 보고) — 주민 픽→팔로우
// 카메라→줌인→풀샷. 스모크(부팅)·플레이 QA(전 패널)과 달리 발 접지 픽셀
// 판독에만 특화. 사용: NODE_PATH=... node tools/e2e/feet_check.cjs [url] [out.png]
const { chromium } = require('playwright');
const URL = process.argv[2] || 'https://terrafolk-3d.pages.dev';
const OUT = process.argv[3] || '/tmp/feet_check.png';

(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: false });
  const pg = await b.newPage({ viewport: { width: 1280, height: 800 } });
  await pg.goto(URL, { waitUntil: 'load', timeout: 90000 });

  // 프라임 대기 — body.tf-loading 해제가 결정 신호(스모크 계약 동일)
  let primed = false;
  for (let i = 0; i < 40 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  console.log('primed:', primed);
  if (!primed) { await b.close(); process.exit(2); }

  await pg.waitForTimeout(1500);
  // 주민 픽 — .tf-wa 앵커 중 화면 중앙대에 가까운 것(ARGV4 index)부터.
  // 물가 주민은 물면 판정이 모호하니[§17.80] 여러 앵커를 돌려가며 찍는다.
  const IDX = parseInt(process.argv[4] || '0', 10);
  const picked = await pg.evaluate((idx) => {
    const els = [...document.querySelectorAll('.tf-wa')].map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + 40, cx: r.x + r.width / 2 };
    }).filter((p) => p.x > 120 && p.x < 1160 && p.y > 120 && p.y < 680)
      .sort((a, b) => Math.abs(a.cx - 640) - Math.abs(b.cx - 640));
    return els[idx] || els[0] || null;
  }, IDX);
  if (picked) {
    await pg.mouse.click(picked.x, picked.y);
    await pg.waitForTimeout(900);
    // 팔로우 카메라 — 패널 hero-cam 버튼
    const hero = await pg.evaluate(() => {
      const el = document.querySelector('[data-act="hero-cam"]');
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    });
    if (hero) { await pg.mouse.click(hero.x, hero.y); await pg.waitForTimeout(1200); }
  }
  // 줌인 — 화면 중앙 휠(플레이 QA와 동일 축: -240=인)
  await pg.mouse.move(640, 500);
  for (let i = 0; i < 6; i++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(300); }
  await pg.waitForTimeout(800);
  await pg.screenshot({ path: OUT });
  console.log('shot:', OUT, 'picked:', JSON.stringify(picked));
  await b.close();
})();
