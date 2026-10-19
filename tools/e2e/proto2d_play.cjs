// proto2d 인터랙티브 플레이 QA — 실제 조작(클릭·드래그·휠)을 단계 스크립트로
// 재생하며 콘솔·네트워크·상태줄·스크린샷 증거를 수집한다.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/proto2d_play.cjs <url> <steps.json> [--headed]
// steps.json = [{shot,click,drag,wheel,key,wait,fps,eval}...] — 좌표는 캔버스 픽셀(뷰포트 동일).
const { chromium } = require('playwright');
const fs = require('fs');

async function proxyRoute(route) {
  const req = route.request();
  const url = req.url();
  const origin = req.headers()['origin'] || '*';
  if (req.method() === 'OPTIONS') {
    return route.fulfill({
      status: 204,
      headers: {
        'access-control-allow-origin': origin,
        'access-control-allow-credentials': 'true',
        'access-control-allow-methods': 'GET,POST,OPTIONS',
        'access-control-allow-headers': 'authorization,content-type',
      },
    });
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
  } catch (e) {
    return route.abort();
  }
}

(async () => {
  const url = process.argv[2];
  const stepsPath = process.argv[3];
  const headed = process.argv.includes('--headed');
  const steps = JSON.parse(fs.readFileSync(stepsPath, 'utf8'));
  const b = await chromium.launch({
    headless: !headed,
    args: [
      '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
      ...(headed ? ['--disable-blink-features=AutomationControlled'] : []),
    ],
  });
  const pg = await b.newPage({ viewport: { width: 1024, height: 768 } });
  const t0 = Date.now();
  const log = [];
  const stamp = () => ((Date.now() - t0) / 1000).toFixed(1) + 's';
  pg.on('console', m => {
    const t = `[${stamp()}][console.${m.type()}]`;
    log.push(`${t} ${m.text().slice(0, 300)}`);
  });
  pg.on('pageerror', e => log.push(`[${stamp()}][pageerror] ${String(e).slice(0, 400)}`));
  pg.on('requestfailed', r =>
    log.push(`[${stamp()}][reqfail] ${r.method()} ${r.url().slice(0, 120)} ${r.failure()?.errorText}`));
  pg.on('response', r => {
    if (r.status() >= 400)
      log.push(`[${stamp()}][http${r.status()}] ${r.request().method()} ${r.url().slice(0, 120)}`);
  });
  await pg.route('**/127.0.0.1:8892/**', proxyRoute);
  await pg.route('**/127.0.0.1:8893/matchmake/**', proxyRoute);

  await pg.goto(url, { waitUntil: 'load', timeout: 30000 });
  for (let i = 0; i < 120; i++) {
    const box = await pg.locator('canvas').boundingBox().catch(() => null);
    if (box && box.width > 300) break;
    await pg.waitForTimeout(500);
  }
  const report = { steps: [] };
  for (const s of steps) {
    const rec = { ...s };
    if (s.wait) await pg.waitForTimeout(s.wait);
    if (s.shot) {
      const path = `/tmp/play/${s.shot}.png`;
      fs.mkdirSync('/tmp/play', { recursive: true });
      await pg.screenshot({ path });
      rec.file = path;
    }
    if (s.click) await pg.mouse.click(s.click[0], s.click[1]);
    if (s.dblclick) await pg.mouse.dblclick(s.dblclick[0], s.dblclick[1]);
    if (s.drag) {
      const [[x0, y0], [x1, y1]] = s.drag;
      await pg.mouse.move(x0, y0);
      await pg.mouse.down();
      const n = 12;
      for (let i = 1; i <= n; i++)
        await pg.mouse.move(x0 + ((x1 - x0) * i) / n, y0 + ((y1 - y0) * i) / n);
      await pg.mouse.up();
    }
    if (s.wheel) await pg.mouse.wheel(s.wheel[0], s.wheel[1]);
    if (s.key) await pg.keyboard.press(s.key);
    if (s.fps) {
      rec.fps = await pg.evaluate(async (secs) => {
        return await new Promise((res) => {
          let n = 0;
          const t0 = performance.now();
          (function f() {
            n++;
            if (performance.now() - t0 < secs * 1000) requestAnimationFrame(f);
            else res((n / secs).toFixed(1));
          })();
        });
      }, s.fps);
    }
    if (s.state) {
      rec.status = ((await pg.locator('#status').textContent().catch(() => '')) || '').slice(0, 160);
      rec.gldiag = ((await pg.locator('#gldiag').textContent().catch(() => '')) || '').slice(0, 80);
      rec.mem = await pg.evaluate(() =>
        performance.memory ? Math.round(performance.memory.usedJSHeapSize / 1048576) + 'MB' : 'n/a');
    }
    report.steps.push(rec);
  }
  fs.writeFileSync('/tmp/play/report.json', JSON.stringify(report, null, 2));
  fs.writeFileSync('/tmp/play/console.log', log.join('\n'));
  console.log(JSON.stringify({ consoleLines: log.length, reportFile: '/tmp/play/report.json' }));
  await b.close();
})().catch(e => { console.error(e); process.exit(1); });
