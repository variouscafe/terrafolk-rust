// 우측 상단 메뉴 UI 게이트(2026-11-04 상용화) — ☰ 버튼·모드 칩·드롭다운·
// bevy HUD 칩(날씨/인구/LIVE)의 레이아웃 붕괘(wrap·겹침) 검출.
// 사용: node tools/e2e/menu_ui.cjs [url]  (기본 http://221.143.21.29:5101 —
// tf-api CORS 허용 오리진. 터널 전송 ~70s 포함 waitPrimed 180s 게이트)
// 판정:
//  ① 데스크톱+모바일에서 메뉴 열람 스크린샷 저장(수목 확인용)
//  ② DOM — ☰ 버튼 ≥30px·드롭다운 항목 6개·구분선 1·모드 헤더 존재
//  ③ 모바일 — 레일 칩 숨김 + 드롭다운 헤더가 모드 표시(2026-11-04 리뷰 8)
//  ④ 상태 점 — TF_UI.setNet(false) 즉시 반영(스케줄 누락 회귀, 리뷰 1)
const { launchLive, waitPrimed } = require('./lib.cjs');

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

      // ③ 드롭다운 구조 — 항목 6·구분선 1·아이콘 칩 6·모드 헤더 1
      const items = await pg.locator('.tf-menu-item').count();
      if (items !== 6) fails.push(`${name}: 메뉴 항목 ${items}개(기대 6)`);
      const seps = await pg.locator('.tf-menu-sep').count();
      if (seps !== 1) fails.push(`${name}: 구분선 ${seps}개(기대 1)`);
      const ico = await pg.locator('.tf-menu-item .tf-menu-ico svg').count();
      if (ico !== 6) fails.push(`${name}: 아이콘 칩 ${ico}개(기대 6)`);
      const heads = await pg.locator('.tf-menu-head').count();
      if (heads !== 1) fails.push(`${name}: 모드 헤더 ${heads}개(기대 1)`);

      // ④ 드롭다운이 화면 우측을 넘지 않는지(우측 여백 ≥ 8px)
      if (await pg.locator('.tf-menu').count()) {
        const m = await pg.locator('.tf-menu').boundingBox();
        if (m && m.x + m.width > vp.width - 8)
          fails.push(`${name}: 드롭다운 화면 우측 초과(x+w=${Math.round(m.x + m.width)}/${vp.width})`);
      }

      // ⑤ 상태 점 즉시 반영 — setNet(false) → 칩·헤더 점이 .off로
      // (리뷰 1: 스케줄 누락 시 프레임이 끊긴 단절 구간에서 갱신 불가)
      await pg.evaluate(() => window.TF_UI && window.TF_UI.setNet(false));
      await pg.waitForTimeout(400);
      const offIn = await pg.locator('.tf-menu-head .dot.off').count();
      if (!offIn) fails.push(`${name}: setNet(false) 후 헤더 점 미변화`);
      await pg.evaluate(() => window.TF_UI && window.TF_UI.setNet(true));

      // ⑥ 모바일 — 레일 모드 칩 숨김(≤600px)+헤더가 모드 텍스트 보유
      if (name === 'mobile') {
        const chip = pg.locator('.tf-chip-mode');
        if ((await chip.count()) && (await chip.first().isVisible()))
          fails.push('mobile: 모드 칩이 숨겨지지 않음');
        const headTxt = await pg.locator('.tf-menu-head span').textContent();
        if (!headTxt || !headTxt.trim()) fails.push('mobile: 헤더 모드 텍스트 비음');
      }
      if (errors.some((e) => /panicked at/.test(e)))
        fails.push(`${name}: Rust panic — ${errors.join(' | ')}`);
    } catch (e) {
      fails.push(`${name}: 게이트 실행 오류 — ${String(e).slice(0, 200)}`);
    } finally {
      await b.close();
    }
  }
  console.log(fails.length ? `FAIL\n- ${fails.join('\n- ')}` : 'PASS');
  console.log(`shots: ${OUT('desktop_closed')} ${OUT('desktop_menu')} ${OUT('mobile_closed')} ${OUT('mobile_menu')}`);
  process.exit(fails.length ? 1 : 0);
})();
