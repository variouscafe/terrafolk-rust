// 집중 재현 프로브 — 발명 상세·주민 클로즈업·선택 해제. (root3d_play 후속)
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_probe.cjs
const { chromium } = require('playwright');
const fs = require('fs');
const OUT = '/tmp/tfplay';
fs.mkdirSync(OUT, { recursive: true });
const errors = [];

(async () => {
  const b = await chromium.launch({
    channel: 'chrome', headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 1280, height: 800 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  pg.on('console', (m) => { const t = m.text(); if (m.type() === 'error' && !/WARN/.test(t)) errors.push('console: ' + t.slice(0, 250)); });
  pg.on('pageerror', (e) => errors.push('pageerror: ' + String(e).slice(0, 250)));

  await pg.goto('https://terrafolk-3d.pages.dev', { waitUntil: 'load', timeout: 30000 });
  for (let i = 0; i < 40 && !(await pg.evaluate(() => !document.body.classList.contains('tf-loading'))); i++)
    await pg.waitForTimeout(3000);
  await pg.waitForTimeout(2000);

  const clickAt = async (sel) => {
    const loc = pg.locator(sel).first();
    await loc.waitFor({ state: 'visible', timeout: 4000 });
    await loc.click();
  };
  const dlgText = () => pg.evaluate(() => {
    const d = document.querySelector('.tf-dialog');
    return d ? d.textContent.replace(/\s+/g, ' ').trim().slice(0, 160) : null;
  });

  // ── A. 발명 상세 — 스프라이트 있는 발명 우선 클릭(실마우스)
  await pg.evaluate(() => {
    document.querySelector('.tf-menu-btn').dispatchEvent(new MouseEvent('click', { bubbles: true }));
  });
  await pg.waitForTimeout(300);
  await clickAt('.tf-menu-item[data-key="inventions"]');
  await pg.waitForTimeout(900);
  const rows = await pg.evaluate(() =>
    Array.from(document.querySelectorAll('.tf-invrow[data-act="sel-inv"]'))
      .map((el) => ({ name: el.getAttribute('data-name'), hasIcon: !!el.querySelector('img, .tf-invicon') }))
  );
  console.log('ROWS:', JSON.stringify(rows.slice(0, 6)));
  const target = rows.find((r) => r.hasIcon) || rows[0];
  if (target) {
    await clickAt(`.tf-invrow[data-name="${target.name.replace(/"/g, '\\"')}"]`);
    await pg.waitForTimeout(1200);
    const detail = await pg.evaluate(() => {
      const d = document.querySelector('.tf-dialog');
      const img = document.querySelector('.tf-invdetail-sprite img');
      return { text: d ? d.textContent.replace(/\s+/g, ' ').trim().slice(0, 160) : null, sprite: !!img, loaded: img ? img.naturalWidth > 0 : false };
    });
    console.log('DETAIL:', JSON.stringify({ target: target.name, ...detail }));
    await pg.screenshot({ path: `${OUT}/p1_inv_detail.png` });
    // 뒤로 가기(close-inv) → 목록 복귀 확인
    const back = await pg.evaluate(() => {
      const el = document.querySelector('[data-act="close-inv"]');
      if (!el) return false;
      el.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      return true;
    });
    await pg.waitForTimeout(600);
    console.log('BACK:', back, 'afterText:', await dlgText());
  }
  // 패널 닫기
  await pg.evaluate(() => { const el = document.querySelector('.tf-dialog-close'); if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true })); });
  await pg.waitForTimeout(400);

  // ── B. 주민 클로즈업 — chars 패널로 선택→팔로우→다이얼로그 닫고 3컷
  await pg.evaluate(() => document.querySelector('.tf-menu-btn').dispatchEvent(new MouseEvent('click', { bubbles: true })));
  await pg.waitForTimeout(300);
  await clickAt('.tf-menu-item[data-key="chars"]');
  await pg.waitForTimeout(800);
  const agent = await pg.evaluate(() => {
    const el = document.querySelector('[data-act="select"]');
    if (!el) return null;
    const id = el.getAttribute('data-id');
    el.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    return id;
  });
  await pg.waitForTimeout(600);
  await pg.evaluate(() => { const el = document.querySelector('[data-act="hero-cam"]'); if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true })); });
  await pg.waitForTimeout(400);
  await pg.evaluate(() => { const el = document.querySelector('.tf-dialog-close'); if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true })); });
  await pg.waitForTimeout(1500);
  await pg.screenshot({ path: `${OUT}/p2_follow_close_a.png` });
  await pg.waitForTimeout(2500);
  await pg.screenshot({ path: `${OUT}/p3_follow_close_b.png` });

  // ── C. 빈 땅 클릭 → 선택 해제 확인(링 소멸)
  await pg.mouse.click(640, 500);
  await pg.waitForTimeout(800);
  await pg.screenshot({ path: `${OUT}/p4_deselect.png` });

  await b.close();
  console.log('ERRORS:', JSON.stringify(errors.slice(0, 8), null, 1));
})();
