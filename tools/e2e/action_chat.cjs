// 채팅 지시 행동 게이트 — 3종 지시(토스 캐치·멀리 던지기·점프) 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/action_chat.cjs [url]
// 기본 http://localhost:8090 (로컬 스택 + dist). 헤디드 필수(스모크 계약 동일).
// 판정: 지시별 [action] 콘솔 로그 시퀀스(발사→캐치/정지, 점프→[grav] 착지) +
// 비행 중 스크린샷. 산출 /tmp/tfact_*.png + JSON 리포트.
const { chromium } = require('playwright');

const OUT = '/tmp';
const shots = [];

async function shot(pg, name) {
  const p = `${OUT}/tfact_${name}.png`;
  await pg.screenshot({ path: p });
  shots.push(p);
  return p;
}

(async () => {
  const url = process.argv[2] || 'http://localhost:8090';
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
  const logs = [];
  const errors = [];
  pg.on('console', (m) => {
    const t = m.text();
    if (t.includes('[action]') || t.includes('[grav]')) logs.push(t.slice(0, 200));
    if (m.type() === 'error') errors.push(t.slice(0, 200));
  });
  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  let primed = false;
  for (let i = 0; i < 40 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  await pg.waitForTimeout(2500);

  // ── 주민 픽 — 이름 있는 앵커 우선(이름 없는 앵커는 경고 배지 등).
  // 세계가 심야(수면)면 몸이 누워 있어 세로 오프셋이 빗나간다 — 가로 그리드
  // 스윕까지 시도한다(2026-10-24 라이브 무음 사건: 수면 몸 픽 실패).
  const sleepWorld = await pg.evaluate(() =>
    Array.from(document.querySelectorAll('.tf-zz')).filter((z) => z.style.display !== 'none').length > 0);
  let picked = false;
  const agentPos = await pg.evaluate(() =>
    Array.from(document.querySelectorAll('.tf-wa')).flatMap((el) => {
      const nm = el.querySelector('.tf-wname');
      if (!nm || !nm.textContent.trim()) return [];
      const r = el.getBoundingClientRect();
      return [{ x: r.x + r.width / 2, y: r.y + r.height / 2 }];
    }),
  );
  if (agentPos.length) {
    agentPos.sort(
      (a, b) => Math.hypot(a.x - 640, a.y - 400) - Math.hypot(b.x - 640, b.y - 400));
    const t = agentPos[0];
    const spots = [];
    for (const dy of [9, 18, 35]) for (const dx of [-30, 0, 30]) spots.push([dx, dy]);
    for (const [dx, dy] of spots) {
      await pg.mouse.click(t.x + dx, t.y + dy);
      await pg.waitForTimeout(650);
      if (await pg.evaluate(() => !!document.querySelector('.tf-wname.sel'))) { picked = true; break; }
    }
  }
  // 히어로 카드는 닫지 않는다 — hero-close가 선택까지 해제한다(sel=null
  // → send() 조기 반환). 이벤트 dispatch 방식이라 카드가 열려 있어도 무방.
  await pg.waitForTimeout(400);

  // ── 지시 3종 — 채팅 입력→Enter(실제 send() 경로: 브리지 발화)
  const directives = [
    // 총 대기 = 행동 총 길이(준비 0.3+비행+정지 1.2s 등)+여유 — active 가드가
    // 수행 중 지시를 드롭하므로 이전 지시 완료를 기다려야 한다(실측 2.9s).
    { key: 'toss', text: '더덕을 던졌다가 받아봐', shotAt: 0.65, wait: 2.2 },
    { key: 'throw', text: '멀리 던져봐', shotAt: 0.70, wait: 3.6 },
    { key: 'jump', text: '점프해봐', shotAt: 0.68, wait: 2.0 },
    // 캐치볼: 3패스 ≈ (준비0.3+비행0.85+홀드0.25)×3+α ≈ 5s — 도중 2컷.
    { key: 'catch', text: '캐치볼 해봐', shotAt: 1.15, wait: 7.5 },
  ];
  const results = {};
  for (const d of directives) {
    const n0 = logs.length;
    const ok = await pg.evaluate((text) => {
      const inp = document.querySelector('.tf-chatdock .tf-input');
      if (!inp) return false;
      inp.value = text;
      inp.dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }),
      );
      return true;
    }, d.text);
    await pg.waitForTimeout(d.shotAt * 1000);
    await shot(pg, d.key);
    if (d.key === 'catch') {
      await pg.waitForTimeout(1600); // 왕복 1회 더 감상
      await shot(pg, 'catch2');
      await pg.waitForTimeout((d.wait - d.shotAt - 1.6) * 1000);
    } else {
      await pg.waitForTimeout((d.wait - d.shotAt) * 1000);
    }
    results[d.key] = { sent: ok, logs: logs.slice(n0) };
  }
  await b.close();

  // ── 판정 — 로그 시퀀스로 물리 사건 확인
  const seq = (arr, ...needles) => needles.every((n) => arr.some((l) => l.includes(n)));
  const tossLogs = results.toss.logs;
  const throwLogs = results.throw.logs;
  const jumpLogs = results.jump.logs;
  const checks = {
    primed,
    picked,
    toss: picked && seq(tossLogs, '지시 수신', '발사 TossCatch', '캐치'),
    throw: picked && seq(throwLogs, '발사 ThrowGround') &&
      throwLogs.some((l) => l.includes('반발') || l.includes('지면 정지')),
    jump: picked && seq(jumpLogs, '점프 vy=', '낙하 착지'),
    catch:
      picked &&
      seq(results.catch ? results.catch.logs : [], '캐치볼 시작', '캐치볼 캐치') &&
      (results.catch ? results.catch.logs : []).filter((l) => l.includes('캐치볼 캐치')).length >= 3,
    noErrors: errors.length === 0,
  };
  const allPass = Object.values(checks).every(Boolean);
  console.log(
    JSON.stringify({ url, allPass, checks, sleepWorld, errors: errors.slice(0, 4), logs, shots }, null, 2),
  );
  process.exit(allPass ? 0 : 1);
})();
