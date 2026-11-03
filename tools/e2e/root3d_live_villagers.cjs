// 3D 클라이언트 라이브 배포 검증 — 주민 렌더·문명 점수(정착 실적) 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/root3d_live_villagers.cjs [url]
// 기본 url = https://terrafolk-3d.pages.dev. 개발 터널 검증은
//   http://221.143.21.29:5101 (tf-api CORS 등록 오리진 — 127.0.0.1:5101은 미등록).
// 라이브(tf-api) 검증은 headed 필수 — suseona.com 존 Cloudflare가 헤드리스
// 지문을 403 차단한다(CLAUDE.md 웹 테스트 계약. root3d_smoke와 동일).
// 판정: (1) 부팅 프라임(body.tf-loading 해제) (2) 캔버스 휘도 분산(스피너 아님)
//       (3) 오버레이 앵커 ≥1(라이브 주민 동기) (4) ☰→문명 팝업 오픈 + 요약
//       텍스트 판독("발명 N · 문명 점수 M" — 정착 실적 미반영이면 구식
//       최대치 N+12를 못 넘는다, 0.8.52+는 넘는다) (5) Rust panic 0.
//       주민 몸체 픽셀 판정은 하지 않는다 — 라벨 앵커+스크린샷 육안으로
//       보완할 것(스크린샷 /tmp/verify_live_world.png·_civ.png 저장).
// 유래: 2026-11-03 롤백 라인(0.8.52) 재배포 — 0.8.6x 무주민 사고 재발 방지.
//       root3d_smoke의 PNG 디코더·런치 패턴 재사용.
const { chromium } = require('playwright');
const fs = require('fs');
const zlib = require('zlib');

/// 최소 PNG 디코더(Playwright 스크린샷 = RGBA8 비인터리이스 전제)
function pngPixels(buf) {
  const b = Buffer.from(buf);
  if (b.readUInt32BE(0) !== 0x89_50_4e_47) throw new Error('not png');
  let off = 8, w = 0, h = 0, idat = [], ctype = 6;
  while (off < b.length) {
    const len = b.readUInt32BE(off);
    const type = b.toString('ascii', off + 4, off + 8);
    if (type === 'IHDR') { w = b.readUInt32BE(off + 8); h = b.readUInt32BE(off + 12); ctype = b[off + 17]; }
    else if (type === 'IDAT') idat.push(b.subarray(off + 8, off + 8 + len));
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
  const PNG_WORLD = process.env.PNG_WORLD || '/tmp/verify_live_world.png';
  const PNG_CIV = process.env.PNG_CIV || '/tmp/verify_live_civ.png';
  const errors = [];
  const b = await chromium.launch({
    channel: 'chrome', headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
  });
  const pg = await b.newPage({
    viewport: { width: 960, height: 680 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36',
  });
  pg.on('console', (m) => {
    const t = m.text();
    if (m.type() === 'error' || /panicked at|RustStdoutStderr/.test(t)) errors.push(t.slice(0, 200));
  });
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  let primed = false, luma = null;
  for (let i = 0; i < 60 && !primed; i++) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading'));
  }
  await pg.waitForTimeout(4000); // 오버레이·주민 정착 여유

  fs.writeFileSync(PNG_WORLD, await pg.screenshot());
  const px = pngPixels(fs.readFileSync(PNG_WORLD));
  luma = boxStats(px, [Math.floor(px.w * 0.2), Math.floor(px.h * 0.35), Math.floor(px.w * 0.8), Math.floor(px.h * 0.85)]);

  const anchors = await pg.evaluate(() => document.querySelectorAll('.tf-wa').length);
  const version = await pg.evaluate(() => {
    const el = document.getElementById('tf-version');
    return el ? el.textContent : '';
  });

  // 문명 팝업 — ☰ → 문명(inventions)
  let civText = '';
  try {
    await pg.click('.tf-menu-btn', { timeout: 5000 });
    await pg.waitForSelector('.tf-menu', { timeout: 5000 });
    await pg.click('.tf-menu-item[data-key="inventions"]', { timeout: 5000 });
    await pg.waitForTimeout(1200);
    civText = await pg.evaluate(() => {
      const el = document.querySelector('.tf-civ-summary-head');
      return el ? el.textContent.replace(/\s+/g, ' ').trim() : '';
    });
    fs.writeFileSync(PNG_CIV, await pg.screenshot());
  } catch (e) {
    errors.push('civ popup: ' + String(e).slice(0, 150));
  }
  await b.close();

  // 정착 실적 판정 — "발명 N · 문명 점수 M"에서 M > N+12(구식 최대 = N+6그룹×2)
  const m = /발명\s*(\d+)\s*·\s*문명 점수\s*(\d+)/.exec(civText || '');
  const invN = m ? Number(m[1]) : -1;
  const score = m ? Number(m[2]) : -1;
  const settleApplied = invN > 0 && score > invN + 12;

  const panic = errors.filter((e) => /panicked at/.test(e));
  const verdict = primed && luma && luma.std > 8 && anchors > 0 && civText && settleApplied && panic.length === 0;
  console.log(JSON.stringify({
    url, version, uiPrimed: primed,
    paintedStd: luma ? +luma.std.toFixed(1) : null,
    overlayAnchors: anchors, civSummary: civText,
    settleApplied, panicCount: panic.length, errSample: errors.slice(0, 4),
  }, null, 2));
  process.exit(verdict ? 0 : 1);
})();
