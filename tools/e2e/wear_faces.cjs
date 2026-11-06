// 전 주민 근접 컷 연사(얼굴 정면 컷 확보용) — beard-full+facial_glasses 강제(setLocalLook) 후 머리 영역 크롭.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/wear_faces.cjs <tag> [shots] → /tmp/wear_faces_<tag>_<id>_<n>.png
const { chromium } = require('playwright');
const { waitPrimed } = require('./lib.cjs');
(async () => {
  const tag = process.argv[2] || 'x'; const shots = +(process.argv[3] || 10);
  const parts = JSON.parse(process.env.TF_PARTS || '{"beard":"beard-full","glasses":"facial_glasses"}');
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const pg = await (await b.newContext({ viewport: { width: 900, height: 700 }, deviceScaleFactor: 1.5 })).newPage();
  await pg.goto('https://terrafolk-3d.pages.dev/?cb=' + Date.now(), { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg); await pg.waitForTimeout(8000);
  const ids = await pg.evaluate(() => Object.keys((window.TF_UI && window.TF_UI.state.live) || {}));
  console.log('ids', ids.join(','));
  for (const id of ids) {
    await pg.evaluate((id) => { window.terrafolk.setLookFocus(false); window.terrafolk.selectAgent(id); window.terrafolk.setLookFocus(true); }, id);
    await pg.waitForTimeout(4000);
    await pg.evaluate(([id, p]) => window.terrafolk.setLocalLook(id, p), [id, JSON.stringify(parts)]);
    await pg.waitForTimeout(6000);
    await pg.evaluate((id) => window.terrafolk.selectAgent(id), id);
    for (let n = 0; n < shots; n++) {
      await pg.screenshot({ path: `/tmp/wear_faces_${tag}_${id}_${n}.png` });
      await pg.waitForTimeout(1500);
    }
  }
  await b.close();
})();
