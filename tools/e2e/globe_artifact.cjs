// 글로브 줌아웃 '파란 단색 조각' 재현·검출기 (작업지시서 docs/globe-blue-chunk-workorder.md 1단계).
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/globe_artifact.cjs <tag> [extraFlags] [phone|desktop|phone2]
//  예) ... base "" phone   /   ... nowater no_water phone
// 출력: /tmp/globe_artifact_<tag>.json, 스크린샷 /tmp/globe_artifact_<tag>_N.png
const { chromium } = require('playwright');
const fs = require('fs');
const { pngPixels, waitPrimed } = require('./lib.cjs');

/// matcolor 계열색 픽셀 수(순색 ±12) — 마젠타(std-unlit)·노랑(std-lit)·초록(툰)·빨강(잉크)·시안(바람)
function classCounts(px) {
  // 톤맵·블룸으로 순색이 눌리므로 채널 우세(포화) 기준: 가장 큰 두 채널 대비 최소 채널 비율로 분류
  const out = { magenta: 0, yellow: 0, green: 0, red: 0, cyan: 0 };
  for (let i = 0; i < px.w * px.h; i++) {
    const o = i * px.ch;
    const r = px.data[o], g = px.data[o + 1], b = px.data[o + 2];
    const mx = Math.max(r, g, b), mn = Math.min(r, g, b);
    if (mx < 90 || mn > mx * 0.35) continue; // 어둡거나 무채색
    const hi = (v) => v > mx * 0.75;
    if (hi(r) && hi(b) && !hi(g)) out.magenta++;
    else if (hi(r) && hi(g) && !hi(b)) out.yellow++;
    else if (hi(g) && hi(b) && !hi(r)) out.cyan++;
    else if (hi(r) && !hi(g) && !hi(b)) out.red++;
    else if (hi(g) && !hi(r) && !hi(b)) out.green++;
  }
  return out;
}

/// 정확색 게이트(분기 V 회귀): 주민 다리 늘어남 판 = (117,165,212)±1 단색. 밤낮 무관.
function exactCount(px) {
  let n = 0;
  for (let i = 0; i < px.w * px.h; i++) {
    const o = i * px.ch;
    if (Math.abs(px.data[o] - 117) <= 1 && Math.abs(px.data[o + 1] - 165) <= 1 && Math.abs(px.data[o + 2] - 212) <= 1) n++;
  }
  return n;
}

const VIEWS = {
  phone: { viewport: { width: 412, height: 732 }, deviceScaleFactor: 2.625 },
  phone2: { viewport: { width: 412, height: 673 }, deviceScaleFactor: 2.625 },
  desktop: { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 },
};

/// 청색 단색 블롭 검출: B>R+60 && B>G+25, 4-연결, 면적>=minArea, RGB std 합<3
function detect(px, minArea = 1500) {
  const { w, h, ch, data } = px;
  const mask = new Uint8Array(w * h);
  for (let i = 0; i < w * h; i++) {
    const r = data[i * ch], g = data[i * ch + 1], b = data[i * ch + 2];
    if (b > r + 60 && b > g + 25) mask[i] = 1;
  }
  const seen = new Uint8Array(w * h);
  const blobs = [];
  const stack = [];
  for (let s = 0; s < w * h; s++) {
    if (!mask[s] || seen[s]) continue;
    const hist = new Map(); let area = 0, x0 = w, y0 = h, x1 = 0, y1 = 0, sr = 0, sg = 0, sb = 0, qr = 0, qg = 0, qb = 0;
    stack.push(s); seen[s] = 1;
    while (stack.length) {
      const p = stack.pop();
      const x = p % w, y = (p / w) | 0;
      area++;
      const r = data[p * ch], g = data[p * ch + 1], b = data[p * ch + 2];
      { const k = (r << 16) | (g << 8) | b; hist.set(k, (hist.get(k) || 0) + 1); }
      sr += r; sg += g; sb += b; qr += r * r; qg += g * g; qb += b * b;
      if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
      for (const q of [x > 0 ? p - 1 : -1, x < w - 1 ? p + 1 : -1, y > 0 ? p - w : -1, y < h - 1 ? p + w : -1])
        if (q >= 0 && mask[q] && !seen[q]) { seen[q] = 1; stack.push(q); }
    }
    if (area < minArea) continue;
    const sd = (sq, sm) => Math.sqrt(Math.max(0, sq / area - (sm / area) ** 2));
    const std = [sd(qr, sr), sd(qg, sg), sd(qb, sb)];
    // 단색 판정: 최빈색 ±1 이내 픽셀 비율(가장자리 안티앨리어싱이 std를 부풀리므로 보조 지표 추가)
    let mk = 0, mc = 0;
    for (const [k, c] of hist) if (c > mc) { mc = c; mk = k; }
    let near = 0;
    for (const [k, c] of hist)
      if (Math.abs((k >> 16) - (mk >> 16)) <= 1 && Math.abs(((k >> 8) & 255) - ((mk >> 8) & 255)) <= 1 && Math.abs((k & 255) - (mk & 255)) <= 1) near += c;
    blobs.push({ area, bbox: [x0, y0, x1, y1], meanRGB: [sr / area, sg / area, sb / area].map((v) => +v.toFixed(1)),
      std: std.map((v) => +v.toFixed(2)), modeRGB: [mk >> 16, (mk >> 8) & 255, mk & 255], modeFrac: +(near / area).toFixed(3),
      flat: std[0] + std[1] + std[2] < 3 || near / area >= 0.7 });
  }
  return blobs;
}

(async () => {
  const tag = process.argv[2] || 'base';
  const extra = process.argv[3] || '';
  const view = VIEWS[process.argv[4] || 'phone'];
  const base = process.env.TF_URL || 'https://terrafolk-3d.pages.dev/';
  const sf = process.argv[5]; // 'S'=글로브 중 스폰(?tfdbg=globe 부팅) · 'F'=평면 스폰 후 줌아웃 (A4-1)
  const flags = sf === 'F' ? [extra] : ['globe', extra];
  const q = flags.filter(Boolean).join(',');
  const url = `${base}?${q ? 'tfdbg=' + q + '&' : ''}cb=${Date.now()}`;
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ ...view, isMobile: false,
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36' });
  const pg = await ctx.newPage();
  const errors = [];
  pg.on('console', (m) => { const t = m.text(); if (m.type() === 'error' || /panicked at|naga|wgpu/.test(t)) errors.push(t.slice(0, 200)); });
  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  const primed = await waitPrimed(pg);
  await pg.waitForTimeout(5000);
  const res = { tag, url, primed, view: process.argv[4] || 'phone', shots: [], errors };
  const snap = async (n) => {
    const f = `/tmp/globe_artifact_${tag}_${n}.png`;
    const buf = await pg.screenshot();
    fs.writeFileSync(f, buf);
    const px = pngPixels(buf);
    const blobs = detect(px);
    // 형태 판정: 화면 높이 30% 초과 세로 길이의 단색 블롭(조명 의존 색 대비)
    const tall = blobs.filter((x) => x.flat && x.bbox[3] - x.bbox[1] > px.h * 0.3 && x.area < px.w * px.h * 0.5);
    res.shots.push({ shot: f, exact: exactCount(px), classes: extra.includes('matcolor') ? classCounts(px) : undefined, tallFlat: tall.length, blobs });
  };
  if (sf) {
    const cx0 = view.viewport.width / 2, cy0 = view.viewport.height / 2;
    await pg.mouse.move(cx0, cy0);
    await pg.waitForTimeout(15000); // 프라임 후 총 20s 정착
    if (sf === 'S') { await pg.mouse.wheel(0, 120); await pg.waitForTimeout(1500); await pg.mouse.wheel(0, -120); await pg.waitForTimeout(2500); }
    else { for (let i = 0; i < 24; i++) { await pg.mouse.wheel(0, -240); await pg.waitForTimeout(120); } await pg.waitForTimeout(4000); }
    for (let i = 1; i <= 3; i++) { await snap(i); await pg.waitForTimeout(2500); }
    if (sf === 'S') { for (let i = 0; i < 24; i++) { await pg.mouse.wheel(0, 240); await pg.waitForTimeout(120); } await pg.waitForTimeout(3000); await snap(4); }
    res.pass = res.shots.every((x) => x.exact < 200);
    fs.writeFileSync(`/tmp/globe_artifact_${tag}.json`, JSON.stringify(res, null, 1));
    console.log(`SF ${sf} ${tag} exact=${res.shots.map((x) => x.exact)}`);
    await b.close();
    return;
  }
  await snap(1);
  const cx = view.viewport.width / 2, cy = view.viewport.height / 2;
  await pg.mouse.move(cx, cy);
  for (let i = 2; i <= 5; i++) {
    await pg.mouse.wheel(0, 120);
    await pg.waitForTimeout(1500);
    await snap(i);
  }
  await pg.mouse.move(cx, cy); await pg.mouse.down();
  await pg.mouse.move(cx + 150, cy, { steps: 10 }); await pg.mouse.up();
  await pg.waitForTimeout(1500);
  await snap(6);
  fs.writeFileSync(`/tmp/globe_artifact_${tag}.json`, JSON.stringify(res, null, 1));
  res.pass = res.shots.every((s) => s.exact < 200);
  fs.writeFileSync(`/tmp/globe_artifact_${tag}.json`, JSON.stringify(res, null, 1));
  console.log(`GATE ${res.pass ? 'PASS' : 'FAIL'} exact=${res.shots.map((s) => s.exact)} tallFlat=${res.shots.map((s) => s.tallFlat)}`);
  const flat = res.shots.flatMap((s) => s.blobs.filter((x) => x.flat));
  console.log(tag, 'primed', primed, 'flatBlobs', flat.length, 'maxArea', Math.max(0, ...flat.map((x) => x.area)),
    JSON.stringify(res.shots.map((s) => s.blobs.map((x) => [x.area, x.meanRGB, x.flat, x.modeRGB, x.modeFrac]))));
  await b.close();
})();
