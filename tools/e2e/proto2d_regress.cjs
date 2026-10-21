// proto2d 회귀 배터리 — 전 패널+채팅을 panelCmds 훅으로 열고 픽셀 판정.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/proto2d_regress.cjs [url]
// 기본 url = http://127.0.0.1:8095 (로컬 라이브 스택 필요: 8892/8893 + dist 서빙).
// 판정: 각 패널 영역 평균 휘도 < 임계 = OPEN. PASS/FAIL 요약 출력.
// 유래: #63(월드 스프라이트)이 플로팅 패널을 전멸시켰는데 발명 아이콘만
// 검증해 배포됨 — 배포 게이트용으로 정식화(2026-10-21).
const { chromium } = require('playwright');
const fs = require('fs');
const zlib = require('zlib');

/// 최소 PNG 디코더(Playwright 스크린샷 = RGBA8 비인터레이스 전제) —
/// 캔버스 readback은 WebGL preserveDrawingBuffer 한계로 공백이 나온다.
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

/// 박스 평균 휘도
function boxMean(px, [x0, y0, x1, y1]) {
  let sum = 0, n = 0;
  for (let y = y0; y < y1; y += 2)
    for (let x = x0; x < x1; x += 2) {
      const i = (y * px.w + x) * px.ch;
      sum += (px.data[i] + px.data[i + 1] + px.data[i + 2]) / 3;
      n++;
    }
  return sum / n;
}

async function proxyRoute(route) {
  const req = route.request();
  const url = req.url();
  const origin = req.headers()['origin'] || '*';
  if (process.env.TF_MOCK_ECO && url.includes('/api/worldsim/economy')) {
    return route.fulfill({
      status: 200,
      headers: { 'content-type': 'application/json', 'access-control-allow-origin': origin },
      body: JSON.stringify({
        prices: [{ item: 'wood', price_g: 1.8, updated_day: 29, trades_seen: 14 }],
        kpi: { day: 29, goldShare: 0.18, volume: 87, goldDeals: 12 },
        recent_deals: [],
      }),
    });
  }
  if (req.method() === 'OPTIONS') {
    return route.fulfill({ status: 204, headers: {
      'access-control-allow-origin': origin,
      'access-control-allow-credentials': 'true',
      'access-control-allow-methods': 'GET,POST,OPTIONS',
      'access-control-allow-headers': 'authorization,content-type',
    }});
  }
  try {
    const init = { method: req.method(), headers: { ...req.headers() } };
    delete init.headers['host'];
    const body = req.postData();
    if (body) init.body = body;
    const r = await fetch(url, init);
    const headers = await r.headers;
    const buf = Buffer.from(await r.arrayBuffer());
    return route.fulfill({
      status: r.status,
      headers: {
        'content-type': headers['content-type'] || 'application/json',
        'access-control-allow-origin': origin,
        'access-control-allow-credentials': 'true',
      },
      body: buf,
    });
  } catch (e) { return route.abort(); }
}

// 패널 판정표 — [이름, cmd, 크롭박스, 임계(평균 휘도)]
// 데스크톱 1024x768 기준 좌표. 카드/패널은 다크 글래스(평균 <110).
const CHECKS = [
  ['civ', 'civ:1', [340, 160, 680, 560], 130],
  ['relations', 'relations:1', [430, 50, 750, 380], 140], // 부팅 카메라가 마을 중심 유지(재중심 제거)로 배경 블렌드 +
  ['economy', 'economy:1', [200, 50, 600, 460], 140],
  ['resources', 'resources:1', [340, 160, 680, 560], 130],
  ['events', 'events:1', [20, 250, 360, 640], 130],
  ['settings', 'settings:1', [400, 250, 620, 450], 140],
];

(async () => {
  const url = process.argv[2] || 'http://127.0.0.1:8095';
  const b = await chromium.launch({
    headless: true,
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
  const pg = await b.newPage({ viewport: { width: 1024, height: 768 } });
  const errors = [];
  pg.on('pageerror', e => errors.push(String(e).slice(0, 120)));
  await pg.route('**/127.0.0.1:8892/**', proxyRoute);
  await pg.route('**/127.0.0.1:8893/**', proxyRoute);
  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  for (let i = 0; i < 24; i++) {
    const box = await pg.locator('canvas').boundingBox().catch(() => null);
    if (box && box.width > 300) break;
    await pg.waitForTimeout(500);
  }
  await pg.waitForTimeout(11000);
  const status = await pg.locator('#status').textContent().catch(() => '');
  const results = [];
  for (const [name, cmd, box, thresh] of CHECKS) {
    await pg.evaluate(c => window.terrafolk.ui.panelCmds.push(c), cmd);
    // 어댑티브 대기 — 저FPS(SwiftShader)에선 panelCmds 훅 드레인이 다음
    // 프레임까지 수 초 걸린다(2026-10-21 실측 최대 7초, 런치 후 감속).
    // 고정 대기 대신 패널 오픈(박스 어두움 < 임계)을 폴링해 기대 없이 통과.
    let dark = 999, tries = 0;
    const shot = `/tmp/play/rg-${name}.png`;
    do {
      await pg.waitForTimeout(700); tries++;
      await pg.screenshot({ path: shot });
      dark = boxMean(pngPixels(fs.readFileSync(shot)), box);
    } while (dark >= thresh && tries < 16);
    const pass = dark >= 0 && dark < thresh;
    results.push({ name, dark: Math.round(dark), thresh, pass, tries });
    await pg.evaluate(c => window.terrafolk.ui.panelCmds.push(c.split(':')[0] + ':0'), cmd);
    await pg.waitForTimeout(300);
  }
  // 채팅 독·주민 패널(Panel 계열) — 부활 확인(월드 스프라이트 사고 패턴)
  const dockShot = await pg.screenshot({ path: '/tmp/play/rg-dock.png' });
  const dockDark = Math.round(boxMean(pngPixels(fs.readFileSync('/tmp/play/rg-dock.png')), [100, 690, 300, 730]));
  results.push({ name: 'chat-dock(Panel)', dark: dockDark, thresh: 130, pass: dockDark < 130 });
  const live = /ag \d+/.test(status) && !/ag 8 ·/.test(status);
  const allPass = results.every(r => r.pass) && errors.length === 0;
  console.log(JSON.stringify({ url, status: status.slice(0, 80), live, errors: errors.length, results, allPass }, null, 1));
  fs.writeFileSync('/tmp/play/regress-last.json', JSON.stringify({ status, results, errors }, null, 1));
  await b.close();
  process.exit(allPass ? 0 : 1);
})().catch(e => { console.error(e); process.exit(2); });
