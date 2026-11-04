// 팔-몸통 결합 검증 — selectAgent로 주민 선택 → 카메라 팔로우로 캐릭터를
// 화면 중앙에 띄워 확대 캡처. 사용: node tools/e2e/arm_gap_check.cjs [url] [tag]
const { launchLive, waitPrimed } = require('./lib.cjs');
const fs = require('fs');

(async () => {
  const base = (process.argv[2] || 'https://terrafolk-3d.pages.dev').replace(/\/$/, '');
  const tag = process.argv[3] || 'check';
  const qs = process.env.QS || '';
  const { b, pg } = await launchLive({ width: 1280, height: 800 });
  try {
    await pg.goto(base + '/' + qs + (qs ? '&' : '?') + 'cb=' + Date.now(), {
      waitUntil: 'load', timeout: 60000,
    });
    if (!(await waitPrimed(pg, 180000))) { console.log('프라임 실패'); process.exitCode = 1; return; }
    await pg.waitForTimeout(5000); // 주민 스폰 완료 대기
    const picked = await pg.evaluate(() => {
      const live = window.TF_UI && window.TF_UI.state.live;
      const ids = live && Object.keys(live);
      if (!ids || !ids.length) return null;
      const id = ids[0];
      window.terrafolk.selectAgent(id);
      return id;
    });
    console.log('selected:', picked);
    if (!picked) { console.log('라이브 주민 없음'); process.exitCode = 1; return; }
    // 카메라 팔로우·히어로 카드 안정 대기 후 중앙 확대 캡처
    await pg.waitForTimeout(6000);
    fs.writeFileSync(`/tmp/arm_${tag}.png`, await pg.screenshot());
    // 중앙 크롭 3배 확대
    const buf = fs.readFileSync(`/tmp/arm_${tag}.png`);
    console.log('saved /tmp/arm_' + tag + '.png');
  } finally {
    await b.close();
  }
})();
