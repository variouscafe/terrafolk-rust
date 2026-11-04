// 드롭다운 라벨 DOM 확증 — 비전 판독 대신 textContent 원문 수집.
// 사용: node tools/e2e/menu_labels.cjs [url]
const { launchLive, waitPrimed } = require('./lib.cjs');
(async () => {
  const url = (process.argv[2] || 'https://terrafolk-3d.pages.dev') + '/?cb=' + Date.now();
  const { b, pg } = await launchLive({ width: 1280, height: 800 });
  await pg.goto(url, { waitUntil: 'load', timeout: 60000 });
  await waitPrimed(pg, 180000);
  await pg.click('.tf-menu-btn');
  await pg.waitForTimeout(400);
  const rows = await pg.$$eval('.tf-menu-item', (els) =>
    els.map((e) => ({
      key: e.getAttribute('data-key'),
      label: (e.querySelector('.tf-menu-txt') || {}).textContent,
      hasIcon: !!e.querySelector('.tf-menu-ico svg'),
      loginCls: e.classList.contains('login'),
    })));
  console.log(JSON.stringify(rows, null, 1));
  await b.close();
})();
