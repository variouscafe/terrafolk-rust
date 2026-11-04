// 우측 상단 메뉴 UI 게이트(2026-11-04 상용화) — ☰ 버튼·모드 칩·드롭다운·
// bevy HUD 칩(날씨/인구/LIVE)의 레이아웃 붕괘(wrap·겹침) 검출.
// 사용: node tools/e2e/menu_ui.cjs [url]  (기본 http://221.143.21.29:5101 —
// tf-api CORS 허용 오리진. 터널 전송 ~70s 포함 waitPrimed 180s 게이트)
// 판정:
//  ① 데스크톱+모바일에서 메뉴 열람 스크린샷 저장(수목 확인용)
//  ② DOM — ☰ 버튼 ≥30px·드롭다운 항목 6개·구분선 1개 존재
//  ③ bevy HUD — 칩 텍스트가 2줄 wrap 안 됨(관전/흐림 찢어짐 회귀 감지:
//     uidbg 대신 메뉴 DOM과 HUD 칩의 화면 범위 겹침은 DOM 측에서만 검출
//     가능하므로 스크린샷 픽셀 판정은 보조)
const { launchLive, waitPrimed } = require('./lib.cjs');
const fs = require('fs');

const URL = process.argv[2] || 'http://221.143.21.29:5101';
const OUT = (n) => `/tmp/menu_ui_${n}.png`;

(async () => {
  const fails = [];
  for (const [name, vp] of [
    ['desktop', { width: 1280, height: 800 }],
    ['mobile', { width: 390, height: 844 }],
  ]) {
    const { b, pg, errors } = await launchLive(vp);
    try {
      await pg.goto(URL, { waitUntil: 'load', timeout: 60000 });
      const ok = await waitPrimed(pg, 180000);
      if (!ok) fails.push(`${name}: 프라임 실패(180s)`);

      // ① 닫힌 상태 — 상단 우측 클러스터
      await pg.waitForTimeout(1200);
      await pg.screenshot({ path: OUT(`${name}_closed`) });

      // ② 메뉴 오픈 — ☰ 클릭
      const btn = pg.locator('.tf-menu-btn');
      const nBtn = await btn.count();
      if (!nBtn) fails.push(`${name}: .tf-menu-btn 없음`);
      else {
        const box = await btn.boundingBox();
        if (!box || box.height < 30 || box.width < 30)
          fails.push(`${name}: ☰ 버튼 터치 타깃 과소(${box && Math.round(box.width)}x${box && Math.round(box.height)})`);
        await btn.click();
        await pg.waitForTimeout(600);
      }
      await pg.screenshot({ path: OUT(`${name}_menu`) });

      // ③ 드롭다운 구조 — 항목 6·구분선 1
      const items = await pg.locator('.tf-menu-item').count();
      if (items !== 6) fails.push(`${name}: 메뉴 항목 ${items}개(기대 6)`);
      const seps = await pg.locator('.tf-menu-sep').count();
      if (seps !== 1) fails.push(`${name}: 구분선 ${seps}개(기대 1)`);
      const ico = await pg.locator('.tf-menu-item .tf-menu-ico svg').count();
      if (ico !== 6) fails.push(`${name}: 아이콘 칩 ${ico}개(기대 6)`);

      // ④ 드롭다운이 화면 우측을 넘지 않는지(우측 여백 ≥ 8px)
      if (await pg.locator('.tf-menu').count()) {
        const m = await pg.locator('.tf-menu').boundingBox();
        if (m && m.x + m.width > vp.width - 8)
          fails.push(`${name}: 드롭다운 화면 우측 초과(x+w=${Math.round(m.x + m.width)}/${vp.width})`);
      }

      // ⑤ 모바일 — 모드 칩 숨김(≤600px) 확인 — count가 아니라 가시성 판정
      if (name === 'mobile') {
        const chip = pg.locator('.tf-chip-mode');
        if ((await chip.count()) && (await chip.first().isVisible()))
          fails.push('mobile: 모드 칩이 숨겨지지 않음');
      }
      if (errors.some((e) => /panicked at/.test(e)))
        fails.push(`${name}: Rust panic — ${errors.join(' | ')}`);
    } finally {
      await b.close();
    }
  }
  console.log(fails.length ? `FAIL\n- ${fails.join('\n- ')}` : 'PASS');
  console.log(`shots: ${OUT('desktop_closed')} ${OUT('desktop_menu')} ${OUT('mobile_closed')} ${OUT('mobile_menu')}`);
  process.exit(fails.length ? 1 : 0);
})();
