// 발 꺾임 조사(2026-10-28 '돌쇠') — 주민 픽→hero-cam→줌인→버스트 8장.
// 패널에서 이름·행동도 읽는다(걷는 주민 판별). 사용:
//   NODE_PATH=... node tools/e2e/feet_burst.cjs [url] [idx] [prefix]
const { chromium } = require('playwright');
const URL = process.argv[2] || 'https://terrafolk-3d.pages.dev';
const IDX = parseInt(process.argv[3] || '0', 10);
const PRE = process.argv[4] || 'feet';
const OUT = `/tmp/${PRE}`;
require('fs').mkdirSync(OUT, { recursive: true });

(async () => {
  const b = await chromium.launch({ channel: 'chrome', headless: false });
  const pg = await b.newPage({ viewport: { width: 1280, height: 800 } });
  await pg.goto(URL, { waitUntil: 'load', timeout: 90000 });
  let primed = false;
  for (let i = 0; i < 40 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  if (!primed) { console.log('NOT PRIMED'); await b.close(); process.exit(2); }
  await pg.waitForTimeout(1500);
  const picked = await pg.evaluate((idx) => {
    const els = [...document.querySelectorAll('.tf-wa')].map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + 40 };
    }).filter((p) => p.x > 120 && p.x < 1160 && p.y > 120 && p.y < 680);
    return els[idx] || els[0] || null;
  }, IDX);
  if (!picked) { console.log('주민 없음'); await b.close(); process.exit(3); }
  await pg.mouse.click(picked.x, picked.y);
  await pg.waitForTimeout(900);
  const info = await pg.evaluate(() => {
    const name = document.querySelector('.tf-hero-name, [class*="hero"] h2, .tf-hero h2');
    const act = document.querySelector('.tf-hero-act-kind, [class*="act-kind"]');
    return { name: name ? name.textContent.trim() : '?', act: act ? act.textContent.trim() : '?' };
  });
  console.log('picked:', JSON.stringify(info));
  const hero = await pg.evaluate(() => {
    const el = document.querySelector('[data-act="hero-cam"]');
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
  });
  if (hero) { await pg.mouse.click(hero.x, hero.y); await pg.waitForTimeout(1200); }
  await pg.mouse.move(640, 500);
  for (let i = 0; i < 8; i++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(250); }
  await pg.waitForTimeout(600);
  for (let i = 0; i < 8; i++) {
    await pg.screenshot({ path: `${OUT}/${String(i).padStart(2, '0')}.png` });
    await pg.waitForTimeout(320);
  }
  console.log('burst:', OUT);
  await b.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
