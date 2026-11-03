// 3D 클라이언트(terrafolk-3d.pages.dev) 배포 스모크 — wasm 부팅·지형 렌더 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_smoke.cjs [url]
// 기본 url = https://terrafolk-3d.pages.dev (pages.dev 자동화 계약 — 커스텀 도메인은 봇 방어 403).
// 판정: 캔버스 영역 휘도 표준편차 > 임계 = 지형 렌더됨(스피너·단색 화면 아님).
const { pngPixels, boxStats } = require('./lib.cjs');
//      Rust panic 콘솔 라인 감지 시 FAIL. 스크린샷 /tmp/root3d_smoke.png 저장.
// 유래: 3D 웹 배포 개시(2026-10-21). PNG 디코더·런치 패턴은 lib.cjs로 분리(2026-11-03).
const { chromium } = require('playwright');
const fs = require('fs');
const zlib = require('zlib');


(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev';
  const PNG_OUT = process.env.PNG_OUT || '/tmp/root3d_smoke.png';
  // 라이브(tf-api) 검증은 headed 필수 — suseona.com 존 Cloudflare가 헤드리스
  // 지문을 403 차단해 CORS 실패로 위장한다(CLAUDE.md 웹 테스트 계약).
  // HEADLESS=1이면 라이브 없는 순수 부팅 검사만(지형 프라임 안 됨 — 데모 스폰이
  // wasm에선 컴파일 아웃, src/world/mod.rs 참조).
  const headed = process.env.HEADLESS !== '1';
  const errors = [];
  const b = await chromium.launch({
    // headed는 시스템 Chrome 채널(플레이wright 캐시에 헤디드 빌드가 없어도 동작).
    // swiftshader는 headless 전용 — headed(실사용자 경로)는 실 GPU로 검증한다.
    ...(headed ? { channel: 'chrome' } : {}),
    headless: !headed,
    args: [
      ...(headed ? [] : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader']),
      ...(headed ? ['--disable-blink-features=AutomationControlled'] : []),
    ],
  });
  const pg = await b.newPage({
    viewport: { width: 900, height: 640 },
    userAgent: headed
      ? 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36'
      : undefined,
  });
  pg.on('console', (m) => {
    const t = m.text();
    if (m.type() === 'error' || /panicked at|RustStdoutStderr/.test(t)) errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });

  // wasm 부팅·첫 프레임 대기 — 결정 신호는 body.tf-loading 해제(Rust tf.uiPrime.
  // 스피너·UI 크롬만으로 휘도 분산이 생겨 오판한 사건 2026-10-21 — 픽셀 판정은 보조).
  // swiftshader 3D 부팅(35MB wasm 컴파일+GPU 초기화)은 느리므로 최대 180s.
  let primed = false, painted = false, last = null, tPrimed = -1;
  for (let i = 0; i < 60 && !(primed && painted); i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
    if (primed && tPrimed < 0) tPrimed = i * 3 + 3;
    const shot = await pg.screenshot();
    fs.writeFileSync(PNG_OUT, shot);
    const px = pngPixels(shot);
    last = boxStats(px, [Math.floor(px.w * 0.2), Math.floor(px.h * 0.35),
                         Math.floor(px.w * 0.8), Math.floor(px.h * 0.85)]);
    // 휘도 표준편차 = 스피너 아님 판정 보조(단독 패스 금지)
    painted = last.std > 8;
  }
  await b.close();

  const panic = errors.filter((e) => /panicked at/.test(e));
  const verdict = primed && painted && panic.length === 0;
  console.log(JSON.stringify({
    url, allPass: verdict, uiPrimed: primed, primedAfterSec: tPrimed, painted,
    luma: last ? { mean: +last.mean.toFixed(1), std: +last.std.toFixed(1) } : null,
    panicCount: panic.length, errSample: errors.slice(0, 5),
  }, null, 2));
  process.exit(verdict ? 0 : 1);
})();
