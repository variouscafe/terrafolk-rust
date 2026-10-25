// 3D 글로브(줌아웃 행성 전환) 게이트 — ?tfdbg=globe 부팅 판정 + 줌 복귀 회귀.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_globe.cjs [url]
// 기본 url = http://localhost:8096 (trunk serve — 로컬 스택 8892/8893 전제).
// 판정 ①행성: 화면 중앙 밴드(행성 실루엣) 휘도 분산 高 + 상단 밴드(하늘) 분산 低
//          = 하늘 한복판 떠 있는 둥근 지형. ②회귀: 휠 줌인 복귀 후 일반 지형 뷰
//          (중앙 밴드 분산 — root3d_smoke와 동일 기준 std>8). panic·셰이더
//          에러 콘솔 감지 시 FAIL. 스크린샷 /tmp/root3d_globe{,_flat}.png.
// 유래: 줌아웃 글로브 전환 구현(2026-10-25) — docs/globe-map-review.md 안 A 게이트.
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

/// 박스 휘도 통계 {mean, std}
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
  const url = process.argv[2] || 'http://localhost:8096/?tfdbg=globe';
  const headed = process.env.HEADLESS !== '1';
  const errors = [];
  const b = await chromium.launch({
    ...(headed ? { channel: 'chrome' } : {}),
    headless: !headed,
    args: [
      ...(headed ? [] : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader']),
      ...(headed ? ['--disable-blink-features=AutomationControlled'] : []),
      // 로컬 trunk 포트(8096 등)는 백엔드 CORS 허용목록(5174 고정) 밖 —
      // 로컬 진단 게이트 전용 우회. 배포(pages.dev) 검증 시엔 무해(이미 허용).
      '--disable-web-security',
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
    if (m.type() === 'error' || /panicked at|RustStdoutStderr|naga|wgpu/.test(t)) errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });

  // 프라임 대기 — body.tf-loading 해제 + 첫 페인트
  let primed = false;
  for (let i = 0; i < 60 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  // 행성 정착 대기(줌 스무딩·워프 안정화)
  await pg.waitForTimeout(2500);
  fs.writeFileSync('/tmp/root3d_globe.png', await pg.screenshot());

  // ── 판정 ②회귀: 휠 줌인으로 평면 뷰 복귀(곡률 램프 6.0→1.0 이하)
  const mid = await pg.locator('canvas').boundingBox();
  const cx = Math.round(mid.x + mid.width / 2), cy = Math.round(mid.y + mid.height / 2);
  for (let i = 0; i < 22; i++) {
    await pg.mouse.move(cx, cy);
    await pg.mouse.wheel(0, 240); // 줌인 방향(WHEEL_ZOOM_BASE^Δ 누적)
    await pg.waitForTimeout(120);
  }
  await pg.waitForTimeout(2000);
  fs.writeFileSync('/tmp/root3d_globe_flat.png', await pg.screenshot());
  await b.close();

  const g = pngPixels(fs.readFileSync('/tmp/root3d_globe.png'));
  const f = pngPixels(fs.readFileSync('/tmp/root3d_globe_flat.png'));
  // 행성 판정 밴드 — 중앙(행성)·후보 3곳 중 최소 분산(= 순수 하늘; 미니맵·
  // LIVE 칩 등 UI 크롬이 어느 모서리에 뜨든 회피)
  const midBand = boxStats(g, [Math.floor(g.w * 0.30), Math.floor(g.h * 0.38),
                               Math.floor(g.w * 0.70), Math.floor(g.h * 0.62)]);
  const corners = [
    boxStats(g, [Math.floor(g.w * 0.04), Math.floor(g.h * 0.14),
                 Math.floor(g.w * 0.18), Math.floor(g.h * 0.24)]),
    boxStats(g, [Math.floor(g.w * 0.84), Math.floor(g.h * 0.16),
                 Math.floor(g.w * 0.96), Math.floor(g.h * 0.26)]),
    boxStats(g, [Math.floor(g.w * 0.42), Math.floor(g.h * 0.10),
                 Math.floor(g.w * 0.58), Math.floor(g.h * 0.16)]),
  ];
  const skyBand = corners.reduce((a, c) => (c.std < a.std ? c : a));
  // 복귀 판정 — 중앙 광역(스모크 기준)
  const flatBand = boxStats(f, [Math.floor(f.w * 0.2), Math.floor(f.h * 0.35),
                                Math.floor(f.w * 0.8), Math.floor(f.h * 0.85)]);

  const globeOk = midBand.std > 8 && skyBand.std < 7 && Math.abs(midBand.mean - skyBand.mean) > 6;
  const flatOk = flatBand.std > 8;
  const panic = errors.filter((e) => /panicked at/.test(e));
  const shaderErr = errors.filter((e) => /naga|wgpu.*(error|invalid)/i.test(e));
  const verdict = primed && globeOk && flatOk && panic.length === 0 && shaderErr.length === 0;
  console.log(JSON.stringify({
    url, allPass: verdict, uiPrimed: primed,
    globe: { ok: globeOk, mid: { mean: +midBand.mean.toFixed(1), std: +midBand.std.toFixed(1) },
             sky: { mean: +skyBand.mean.toFixed(1), std: +skyBand.std.toFixed(1) } },
    flat: { ok: flatOk, std: +flatBand.std.toFixed(1) },
    panicCount: panic.length, shaderErrCount: shaderErr.length,
    errSample: errors.slice(0, 5),
  }, null, 2));
  process.exit(verdict ? 0 : 1);
})();
