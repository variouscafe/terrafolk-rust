// 착용품(정적 슬롯) 근접 컷 캘리브레이션 — selectAgent + setLookFocus + setLocalLook(parts) 결정적 근접.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/wear_closeup.cjs [tag]
// 출력 /tmp/wear_closeup_<tag>_<agent M|F>_<case>_<n>.png (case: beard·glasses·both·none)
const { chromium } = require('playwright');
const { waitPrimed } = require('./lib.cjs');
const CASES = {
  none: { hair: 'hair-parted', beard: 'none', glasses: 'none' },
  beard: { hair: 'hair-parted', beard: 'beards_beard', glasses: 'none' },
  glasses: { hair: 'hair-parted', beard: 'none', glasses: 'facial_glasses_round' },
  both: { hair: 'hair-parted', beard: 'beards_beard', glasses: 'facial_glasses_round' },
};
(async () => {
  const tag = process.argv[2] || 'x';
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ viewport: { width: 900, height: 700 }, deviceScaleFactor: 1.5 });
  const pg = await ctx.newPage();
  await pg.goto('https://terrafolk-3d.pages.dev/?cb=' + Date.now(), { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg);
  await pg.waitForTimeout(8000);
  const picks = await pg.evaluate(() => {
    const live = (window.TF_UI && window.TF_UI.state.live) || {};
    const out = { sample: null, M: null, F: null };
    for (const id of Object.keys(live)) {
      const a = live[id];
      if (!out.sample) out.sample = JSON.stringify(a).slice(0, 400);
      let sex = null;
      try { sex = JSON.parse(a.look).sex; } catch (e) { sex = a.sex || null; }
      const k = sex && /^[mM남]/.test(sex) ? 'M' : 'F';
      if (!out[k]) out[k] = id;
    }
    return out;
  });
  console.log('picks', JSON.stringify(picks));
  for (const who of ['M', 'F']) {
    const id = picks[who];
    if (!id) continue;
    await pg.evaluate((id) => { window.terrafolk.selectAgent(id); window.terrafolk.setLookFocus(true); }, id);
    await pg.waitForTimeout(4000);
    for (const [name, parts] of Object.entries(CASES)) {
      await pg.evaluate(([id, p]) => window.terrafolk.setLocalLook(id, JSON.stringify(p)), [id, parts]);
      await pg.waitForTimeout(6000);
      // 주민이 걷는 중이라 프레임이 흔들림 — 짧은 간격 연사 후 사람이(Read) 정면 컷을 고른다.
      await pg.evaluate((id) => window.terrafolk.selectAgent(id), id);
      for (let n = 0; n < 8; n++) {
        await pg.screenshot({ path: `/tmp/wear_closeup_${tag}_${who}_${name}_${n}.png` });
        await pg.waitForTimeout(600);
      }
    }
  }
  await b.close();
})();
