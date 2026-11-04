// 팔-몸통 결합 A/B 검증 — 같은 배포에서 shoulder 기본(1.0) vs tfdbg 오버라이드.
// 사용: node tools/e2e/shoulder_ab.cjs [url]  (기본 라이브)
// 산출: /tmp/sh_{tag}.png 전체 화면 — diff로 팔 위치 변화 확인.
const { launchLive, waitPrimed } = require('./lib.cjs');
const fs = require('fs');

(async () => {
  const base = (process.argv[2] || 'https://terrafolk-3d.pages.dev').replace(/\/$/, '');
  for (const [tag, qs] of [
    ['fix', ''],
    ['old', '?tfdbg=shoulder2.2'],
    ['mid', '?tfdbg=shoulder1.4'],
  ]) {
    const { b, pg } = await launchLive({ width: 1280, height: 800 });
    try {
      await pg.goto(base + '/' + qs + (qs ? '&' : '?') + 'cb=' + Date.now(), {
        waitUntil: 'load', timeout: 60000,
      });
      const ok = await waitPrimed(pg, 180000);
      if (!ok) { console.log(`${tag}: 프라임 실패`); process.exitCode = 1; continue; }
      await pg.waitForTimeout(4000); // 주민 스폰·착지 안정
      fs.writeFileSync(`/tmp/sh_${tag}.png`, await pg.screenshot());
      console.log(`${tag}: saved`);
    } finally {
      await b.close();
    }
  }
})();
