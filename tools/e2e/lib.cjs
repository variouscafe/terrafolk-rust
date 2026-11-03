// E2E 공용 라이브러리 — PNG 디코더·휘도 통계·라이브 헤디드 런처(2026-11-03,
// 재사용 관점 분리). root3d_smoke·root3d_live_villagers 등이 같은 판정 코드를
// 복제해 왔던 것을 하나로. 사용: const { pngPixels, boxStats, launchLive } =
// require('./lib.cjs');
const { chromium } = require('playwright');

/// 최소 PNG 디코더(Playwright 스크린샷 = RGBA8 비인터리이스 전제)
function pngPixels(buf) {
  const b = Buffer.from(buf);
  if (b.readUInt32BE(0) !== 0x89_50_4e_47) throw new Error('not png');
  let off = 8, w = 0, h = 0, idat = [], ctype = 6;
  while (off < b.length) {
    const len = b.readUInt32BE(off);
    const type = b.toString('ascii', off + 4, off + 8);
    if (type === 'IHDR') {
      w = b.readUInt32BE(off + 8);
      h = b.readUInt32BE(off + 12);
      ctype = b[off + 17];
    } else if (type === 'IDAT') idat.push(b.subarray(off + 8, off + 8 + len));
    else if (type === 'IEND') break;
    off += 12 + len;
  }
  const raw = require('zlib').inflateSync(Buffer.concat(idat));
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

/// 라이브(tf-api) 검증용 헤디드 런처 — suseona.com 존 Cloudflare가 헤드리스
/// 지문을 403 차단해 CORS 실패로 위장한다(CLAUDE.md 웹 테스트 계약).
/// 시스템 Chrome 채널 + 실제 UA + AutomationControlled 비활성.
/// errors 배열에 console error·pageerror·panic 라인을 모은다.
async function launchLive(viewport = { width: 960, height: 680 }) {
  const errors = [];
  const b = await chromium.launch({
    channel: 'chrome',
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport,
    userAgent:
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  pg.on('console', (m) => {
    const t = m.text();
    if (m.type() === 'error' || /panicked at|RustStdoutStderr/.test(t))
      errors.push(t.slice(0, 300));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
  return { b, pg, errors };
}

/// 부팅 프라임 대기 — 결정 신호는 body.tf-loading 해제(Rust tf.uiPrime).
/// 최대 180s(swasm 컴파일+GPU 초기화). 프라임 여부를 반환.
async function waitPrimed(pg, timeoutMs = 180000) {
  const t0 = Date.now();
  let primed = false;
  while (Date.now() - t0 < timeoutMs && !primed) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  return primed;
}

module.exports = { pngPixels, boxStats, launchLive, waitPrimed };
