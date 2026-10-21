// 3D 클라이언트(terrafolk-3d.pages.dev) 배포 스모크 — wasm 부팅·지형 렌더 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_smoke.cjs [url]
// 기본 url = https://terrafolk-3d.pages.dev (pages.dev 자동화 계약 — 커스텀 도메인은 봇 방어 403).
// 판정: 캔버스 영역 휘도 표준편차 > 임계 = 지형 렌더됨(스피너·단색 화면 아님).
//      Rust panic 콘솔 라인 감지 시 FAIL. 스크린샷 /tmp/root3d_smoke.png 저장.
// 유래: 3D 웹 배포 개시(2026-10-21) — proto2d_regress.cjs의 PNG 디코더·런치 패턴 재사용.
const { chromium } = require('playwright');
const fs = require('fs');
const zlib = require('zlib');

/// 최소 PNG 디코더(Playwright 스크린샷 = RGBA8 비인터레이스 전제)
function pngPixels(buf) {
  const b = Buffer.from(buf);
  if (b.readUInt32BE(0) !== 0x89504e47) throw new Error('not png');
  let off = 8, w = 0, h = 0, idat = [], ctype = 6;
  while (off < b.length) {
    const len = b.readUInt32BE(off);
    const type = b.toString('ascii', off + 4, off + 8);
    if (type === 'IHDR') {
      w = b.readUInt32BE(off + 8); h = b.readUInt32BE(off + 12); ctype = b[off + 17];
    } else if (type === 'IDAT') idat.push(b.subarray(off + 8, off + 8 + len));
    else if (type === 'IEND') break;
    off += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const ch = ctype === 6 ? 4 : 3;
  const stride = w * ch;
  const out = Buffer.alloc(h * stride);
  let p = 0;
  for (let y = 0; y < h; y++) {
    const f = raw[p++];
    const row = raw.subarray(p, p + stride); p += stride;
    const prev = y ? out.subarray((y - 1) * stride, y * stride) : null;
    const cur = out.subarray(y * stride, (y + 1) * stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= ch ? cur[x - ch] : 0;
      const bb = prev ? prev[x] : 0;
      const c = x >= ch && prev ? prev[x - ch] : 0;
      let v = row[x];
      if (f === 1) v += a;
      else if (f === 2) v += bb;
      else if (f === 3) v += (a + bb) >> 1;
      else if (f === 4) {
        const pa = Math.abs(bb - c), pb = Math.abs(a - c), pc = Math.abs(a + bb - 2 * c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? bb : c;
      }
      cur[x] = v & 0xff;
    }
  }
  return { w, h, ch, data: out };
}

/// 박스 휘도 통계 {mean, std} — 지형·소품이 그려졌으면 분산 발생
function boxStats(px, [x0, y0, x1, y1]) {
  let sum = 0, sum2 = 0, n = 0;
  for (let y = y0; y < y1; y += 2)
    for (let x = x0; x < x1; x += 2) {
      const i = (y * px.w + x) * px.ch;
      const l = (px.data[i] + px.data[i + 1] + px.data[i + 2]) / 3;
      sum += l; sum2 += l * l; n++;
    }
  const mean = sum / n;
  return { mean, std: Math.sqrt(Math.max(0, sum2 / n - mean * mean)) };
}

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
