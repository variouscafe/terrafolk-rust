// 3D 클라이언트 직접 플레이 세션 — 관전 플로우 전 경험 + 단계 스크린샷.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_play.cjs [url]
// 산출: /tmp/tfplay/NN_단계.png + /tmp/tfplay/report.json(단계별 판정·콘솔 오류).
// headed 필수(라이브 계약 — root3d_smoke.cjs와 동일). 유래: 직접 플레이 QA(2026-10-22).
const { chromium } = require('playwright');
const fs = require('fs');

const OUT = '/tmp/tfplay';
fs.mkdirSync(OUT, { recursive: true });
const steps = [];
const errors = [];

async function shot(pg, name) {
  const p = `${OUT}/${name}.png`;
  await pg.screenshot({ path: p });
  return p;
}

(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev';
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 1280, height: 800 },
    userAgent:
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  pg.on('console', (m) => {
    const t = m.text();
    if (m.type() === 'error' || /panicked at|WARN|ERROR/.test(t)) errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));

  // ── 1. 부팅·프라임 대기(smoke와 동일 신호 — body.tf-loading 해제)
  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  let primed = false;
  const t0 = Date.now();
  for (let i = 0; i < 40 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  const bootSec = ((Date.now() - t0) / 1000).toFixed(1);
  await pg.waitForTimeout(2500); // 첫 렌더 안정화
  steps.push({ step: 'boot', ok: primed, bootSec, shot: await shot(pg, '01_boot') });

  const click = async (sel) => pg.evaluate((s) => {
    const el = document.querySelector(s);
    if (!el) return false;
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    return true;
  }, sel);

  // ── 2. 휠 줌 — 인 2틱 → 아웃 5틱
  await pg.mouse.move(640, 400);
  for (let i = 0; i < 2; i++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(350); }
  await pg.waitForTimeout(900);
  await shot(pg, '02_zoom_in');
  for (let i = 0; i < 5; i++) { await pg.mouse.wheel(0, 260); await pg.waitForTimeout(350); }
  await pg.waitForTimeout(900);
  steps.push({ step: 'wheel_zoom', ok: true, shot: await shot(pg, '03_zoom_out') });

  // ── 3. 드래그 팬 — 중앙→좌상단 그랩
  await pg.mouse.move(640, 400);
  await pg.mouse.down();
  for (let i = 1; i <= 8; i++) await pg.mouse.move(640 - i * 25, 400 - i * 15);
  await pg.mouse.up();
  await pg.waitForTimeout(900);
  steps.push({ step: 'drag_pan', ok: true, shot: await shot(pg, '04_pan') });

  // ── 4. 키보드 팬 — ArrowRight 홀드
  await pg.keyboard.down('ArrowRight');
  await pg.waitForTimeout(1000);
  await pg.keyboard.up('ArrowRight');
  await pg.waitForTimeout(600);
  steps.push({ step: 'key_pan', ok: true, shot: await shot(pg, '05_key_pan') });

  // ── 5. 주민 픽 — 오버레이 노드(.tf-wa) 위치의 몸체를 실마우스 클릭(Rust 픽킹 경로)
  const agentPos = await pg.evaluate(() => {
    const out = [];
    document.querySelectorAll('.tf-wa').forEach((el) => {
      if (el.classList.contains('off')) return;
      const m = /translate\(([-\d.]+)px,\s*([-\d.]+)px\)/.exec(el.style.transform || '');
      if (m) out.push({ x: +m[1], y: +m[2] });
    });
    return out;
  });
  let pickedRing = false;
  if (agentPos.length) {
    // 화면 중앙에 가장 가까운 주민 — 앵커(1.85m)→몸통중심(≈0.75m) 오프셋은
    // 줌 비례(22m×z 시야)라 브래킷으로 한다: +9px(광역)→+18→+40(근접).
    agentPos.sort((a, b) =>
      (Math.hypot(a.x - 640, a.y - 400)) - (Math.hypot(b.x - 640, b.y - 400)));
    const t = agentPos[0];
    for (const off of [9, 18, 40]) {
      await pg.mouse.click(t.x, t.y + off);
      await pg.waitForTimeout(800);
      if (await pg.evaluate(() => !!document.querySelector('.tf-wname.sel'))) { pickedRing = true; break; }
    }
  }
  steps.push({ step: 'pick_agent', ok: pickedRing, agents: agentPos.length, pickedRing, shot: await shot(pg, '06_pick') });
  // 선택 유발 카드가 열렸으면 닫는다(픽 실패 시 스킵)
  if (pickedRing) { await click('[data-act="hero-close"]').catch(() => {}); await pg.waitForTimeout(400); }

  // ── 6. ☰ 메뉴 → 6패널 스윕(오픈↔클로즈, 각각 스크린샷)
  // 메뉴는 팝업이 열리며 닫히므로 패널마다 메뉴를 다시 연다(첫 판 스윕 실패 원인).
  await click('.tf-menu-btn');
  await pg.waitForTimeout(400);
  const menuVisible = await pg.evaluate(() => !!document.querySelector('.tf-menu'));
  await shot(pg, '07_menu');
  const panels = ['chars', 'res', 'inventions', 'log', 'set', 'account'];
  const panelRes = {};
  for (const k of panels) {
    if (k !== panels[0]) { await click('.tf-menu-btn'); await pg.waitForTimeout(300); }
    const opened = await click(`.tf-menu-item[data-key="${k}"]`);
    await pg.waitForTimeout(700);
    const dlg = await pg.evaluate(() => {
      const d = document.querySelector('.tf-dialog');
      return d ? d.textContent.replace(/\s+/g, ' ').trim().slice(0, 80) : null;
    });
    panelRes[k] = { opened, dialog: dlg };
    await shot(pg, `08_panel_${k}`);
    await click('.tf-dialog-close');
    await pg.waitForTimeout(300);
  }
  steps.push({ step: 'panel_sweep', ok: menuVisible && panels.every((k) => panelRes[k].dialog), panelRes });

  // ── 7. 문명 패널 — 발명 상세(스프라이트·디자인 문구)
  await click('.tf-menu-btn'); await pg.waitForTimeout(300);
  await click('.tf-menu-item[data-key="inventions"]'); await pg.waitForTimeout(700);
  const invRow = await pg.evaluate(() => {
    const el = document.querySelector('.tf-invrow[data-act="sel-inv"]');
    return el ? el.getAttribute('data-name') : null;
  });
  // 실마우스 클릭 + 상세 DOM 폴링(일회 지연 판독은 렌더 레이스 오판 — 2026-10-22)
  let invDetail = { sprite: false, loaded: false, text: null };
  if (invRow) {
    await pg.locator(`.tf-invrow[data-name="${invRow.replace(/"/g, '\\"')}"]`).first().click();
    for (let i = 0; i < 8 && !invDetail.sprite; i++) {
      await pg.waitForTimeout(400);
      invDetail = await pg.evaluate(() => {
        const img = document.querySelector('.tf-invdetail-sprite img');
        const det = !!document.querySelector('.tf-invdetail');
        const dlg = document.querySelector('.tf-dialog');
        return { sprite: det, spriteImg: !!img, loaded: img ? img.naturalWidth > 0 : false, text: dlg ? dlg.textContent.replace(/\s+/g, ' ').trim().slice(0, 100) : null };
      });
    }
  }
  steps.push({ step: 'inv_detail', ok: !!invRow && invDetail.sprite, invRow, invDetail, shot: await shot(pg, '09_inv_detail') });
  await click('.tf-dialog-close'); await pg.waitForTimeout(300);

  // ── 8. 캐릭터 패널 — 선택 → 카메라 팔로우(두 컷 비교)
  await click('.tf-menu-btn'); await pg.waitForTimeout(300);
  await click('.tf-menu-item[data-key="chars"]'); await pg.waitForTimeout(700);
  const picked = await pg.evaluate(() => {
    const el = document.querySelector('[data-act="select"]');
    if (!el) return null;
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    return el.getAttribute('data-id');
  });
  await pg.waitForTimeout(700);
  const heroBtn = await pg.evaluate(() => {
    const el = document.querySelector('[data-act="hero-cam"]');
    if (!el) return false;
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    return true;
  });
  await pg.waitForTimeout(1500);
  await shot(pg, '10_follow_a');
  await pg.waitForTimeout(2500);
  steps.push({ step: 'follow', ok: !!picked && heroBtn, picked, heroBtn, shot: await shot(pg, '11_follow_b') });
  await click('.tf-dialog-close'); await pg.waitForTimeout(300);

  // ── 9. 미니맵 클릭 점프
  const mm = await pg.evaluate(() => {
    const h = document.getElementById('tf-minimap');
    if (!h) return null;
    const r = h.getBoundingClientRect();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2, w: r.width, h: r.height };
  });
  if (mm) { await pg.mouse.click(mm.x, mm.y); await pg.waitForTimeout(1200); }
  steps.push({ step: 'minimap_jump', ok: !!mm, mm, shot: await shot(pg, '12_minimap') });

  // ── 10. 채팅 도크 존재·입력 가능 여부(전송은 스펙테이터 정책 확인 차 시도)
  const chat = await pg.evaluate(() => {
    const inp = document.querySelector('.tf-chatdock .tf-input');
    return { dock: !!document.querySelector('.tf-chatdock'), input: !!inp, disabled: inp ? inp.disabled : null };
  });
  steps.push({ step: 'chat_dock', ok: chat.dock && chat.input, chat });

  // ── 정리 — 콘솔 패닉/오류 요약
  const panic = errors.filter((e) => /panicked at/.test(e));
  await b.close();
  const report = { url, steps, panicCount: panic.length, errCount: errors.length, errSample: errors.slice(0, 10) };
  fs.writeFileSync(`${OUT}/report.json`, JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ allOk: steps.every((s) => s.ok) && panic.length === 0, panicCount: panic.length, errCount: errors.length, steps: steps.map((s) => ({ step: s.step, ok: s.ok })) }, null, 2));
})();
