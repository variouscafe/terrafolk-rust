// §17.121 라이브 프로브 — ko 로케일로 접속해 5분간 말풍선·채팅 독·토스트의 "영문 비율"과
// /translate 요청 수·평균 지연을 JSON으로 출력한다. (배포 후 운영자가 실행 — 헤디드 Chrome)
// 실행: node tools/e2e/chat_mt_live.cjs [url] [out.json] [locale=ko] [minutes=5]
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const url = process.argv[2] || 'https://terrafolk-3d.pages.dev/';
  const out = process.argv[3] || 'chat_mt_live.json';
  const locale = process.argv[4] || 'ko';
  const minutes = Number(process.argv[5] || 5);
  const b = await chromium.launch({ channel: 'chrome', headless: false, args: ['--disable-blink-features=AutomationControlled'] });
  const ctx = await b.newContext({ viewport: { width: 1280, height: 800 }, locale: locale + '-' + locale.toUpperCase(),
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36' });
  await ctx.addInitScript((loc) => {
    try { localStorage.setItem('ws.locale', loc); } catch (e) {}
    // 말풍선·토스트는 수명이 짧아 1초 간격 폴링으로 고유 문장을 누적한다
    window.__seen = { bubble: {}, toast: {} };
    setInterval(() => {
      document.querySelectorAll('.tf-wbubble').forEach((e) => { const t = e.textContent.trim(); if (t && !window.__seen.bubble[t]) window.__seen.bubble[t] = Date.now(); });
      document.querySelectorAll('.tf-toast').forEach((e) => { const t = e.textContent.trim(); if (t && !window.__seen.toast[t]) window.__seen.toast[t] = Date.now(); });
    }, 500);
  }, locale);
  const pg = await ctx.newPage();
  const reqs = []; const errors = [];
  pg.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
  pg.on('console', (m) => { if (m.type() === 'error') errors.push('console: ' + m.text().slice(0, 200)); });
  pg.on('request', (r) => { if (r.url().includes('/translate')) r._t0 = Date.now(); });
  pg.on('requestfinished', async (r) => {
    if (!r.url().includes('/translate')) return;
    let body = {}; try { body = JSON.parse(r.postData() || '{}'); } catch (e) {}
    let resp = null; try { resp = await (await r.response()).json(); } catch (e) {}
    reqs.push({ ms: Date.now() - (r._t0 || Date.now()), target: body.target, n: (body.texts || []).length,
      sample: (body.texts || []).slice(0, 2), got: resp ? Object.keys(resp.texts || {}).length : -1 });
  });
  pg.on('requestfailed', (r) => { if (r.url().includes('/translate')) reqs.push({ failed: r.failure() && r.failure().errorText }); });
  const t0 = Date.now();
  await pg.goto(url, { waitUntil: 'domcontentloaded', timeout: 120000 });
  let primed = false;
  while (Date.now() - t0 < 200000 && !primed) {
    await pg.waitForTimeout(3000);
    primed = await pg.evaluate(() => !document.body.classList.contains('tf-loading')).catch(() => false);
  }
  await pg.evaluate(() => { const d = document.querySelector('.tf-chatdock'); if (d) d.classList.add('on'); }).catch(() => {});
  const dock = new Set();
  const end = Date.now() + minutes * 60000;
  while (Date.now() < end) {
    await pg.waitForTimeout(15000);
    const rows = await pg.evaluate(() => [...document.querySelectorAll('.tf-chatdock .tx')].map((e) => e.textContent.trim()).filter(Boolean).slice(-12)).catch(() => []);
    rows.forEach((s) => dock.add(s));
  }
  const seen = await pg.evaluate(() => ({ bubble: Object.keys(window.__seen.bubble), toast: Object.keys(window.__seen.toast) })).catch(() => ({ bubble: [], toast: [] }));
  // 영문 판정 — 목표 로케일 문자(한글/가나/한자)보다 라틴이 많으면 EN. 발화 태그·주민 이름은 라틴이 적어 영향 작음.
  const script = { ko: /[가-힣]/g, ja: /[぀-ヿ一-鿿]/g, zh: /[一-鿿]/g }[locale];
  const isEn = (s) => (script ? (s.match(/[A-Za-z]/g) || []).length > (s.match(script) || []).length : false);
  const stat = (arr) => ({ n: arr.length, en: arr.filter(isEn).length, en_ratio: arr.length ? +(arr.filter(isEn).length / arr.length).toFixed(3) : null, samples_en: arr.filter(isEn).slice(0, 5) });
  const ok = reqs.filter((r) => r.ms != null);
  const report = {
    locale, primed, minutes, boot_s: Math.round((Date.now() - t0) / 1000),
    bubbles: stat(seen.bubble), dock: stat([...dock]), toasts: stat(seen.toast),
    translate: { requests: reqs.length, failed: reqs.filter((r) => r.failed).length,
      avg_ms: ok.length ? Math.round(ok.reduce((a, r) => a + r.ms, 0) / ok.length) : null,
      max_ms: ok.length ? Math.max(...ok.map((r) => r.ms)) : null,
      total_texts: ok.reduce((a, r) => a + (r.n || 0), 0), reqs: reqs.slice(0, 40) },
    errors: errors.slice(0, 30),
  };
  await pg.screenshot({ path: out.replace(/\.json$/, '.png') });
  fs.writeFileSync(out, JSON.stringify(report, null, 1));
  console.log(JSON.stringify({ bubbles: report.bubbles, dock: report.dock, toasts: report.toasts, translate: { ...report.translate, reqs: undefined } }, null, 1));
  await b.close();
})();
