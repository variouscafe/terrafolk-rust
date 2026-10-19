// proto2d 라이브 E2E — 로컬 서빙 dist + 로컬 백엔드(8892/8893) 라이브 프라임 검증.
// 로컬 백엔드 CORS 허용 목록은 5174(레거시 Vite)만 있어 임의 포트 서빙이 막히므로,
// 티켓 GET·matchmake POST만 Playwright 라우팅으로 노드 프록시(ACAO 헤더 부여)한다.
// WebSocket은 브라우저 CORS 대상이 아니라 직접 연결. prod 검증은 --headed로.
// 사용: NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/proto2d_live.cjs [url] [--headed]
const { chromium } = require('playwright');

async function proxyRoute(route) {
  const req = route.request();
  const url = req.url();
  const origin = req.headers()['origin'] || '*';
  // 프리플라이트 — 인증 헤더(Authorization) 때문에 필요. 룸 서버는 405로
  // 씹으므로 프록시가 직접 승인한다(와일드카드+credentials 조합은 브라우저가 거부).
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
    const init = {
      method: req.method(),
      headers: { ...req.headers() },
    };
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
  const url = process.argv[2] || 'http://127.0.0.1:8095';
  const headed = process.argv.includes('--headed');
  const b = await chromium.launch({
    headless: !headed,
    args: [
      '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
      ...(headed ? ['--disable-blink-features=AutomationControlled'] : []),
    ],
  });
  const pg = await b.newPage({ viewport: { width: 900, height: 700 } });
  // CORS 우회 프록시 — 로컬 백엔드 HTTP 엔드포인트
  await pg.route('**/127.0.0.1:8892/**', proxyRoute);
  await pg.route('**/127.0.0.1:8893/matchmake/**', proxyRoute);
  pg.on('pageerror', e => console.log('[pageerror]', String(e).slice(0, 200)));

  await pg.goto(url, { waitUntil: 'networkidle' });
  for (let i = 0; i < 120; i++) {
    const box = await pg.locator('canvas').boundingBox().catch(() => null);
    if (box && box.width > 300) break;
    await pg.waitForTimeout(500);
  }
  await pg.waitForTimeout(10000); // 프라임+첫 패치+스폰 여유

  const status = await pg.locator('#status').textContent().catch(() => '');
  const hasMap = await pg.evaluate(() =>
    Promise.resolve(typeof window.TF_MAP_JSON === 'string'));
  const shot = '/tmp/proto2d_e2e.png';
  await pg.screenshot({ path: shot });
  console.log(JSON.stringify({ status: (status || '').slice(0, 140), hasMap, shot }));
  await b.close();
})().catch(e => { console.error(e); process.exit(1); });
