// §17.121 채팅 표시 규칙·파라미터 단위 테스트 — 브라우저 없이 vm으로 tf-data + tf-i18n 로드.
// 실행: node tools/e2e/i18n_chat_unit.cjs  → 마지막 줄 "OK n checks"
const vm = require('vm');
const fs = require('fs');
const path = require('path');
const dir = path.join(__dirname, '..', '..', 'assets', 'web');
const win = { terrafolk: {}, TF: { apiBase: '' } };
const ctx = {
  window: win, console, setTimeout, clearTimeout, Promise, Map, Set, Object, Array, JSON,
  navigator: { language: 'en-US' },
  document: { documentElement: {}, title: '' },
  localStorage: { getItem() { return null; }, setItem() {} },
  fetch: () => Promise.reject(new Error('no network')),
};
win.window = win;
vm.createContext(ctx);
ctx.window = win; Object.assign(win, ctx);
for (const f of ['tf-data.js', 'tf-i18n.js']) vm.runInContext(fs.readFileSync(path.join(dir, f), 'utf8'), ctx, { filename: f });
const I = win.TF_I18N;
let n = 0;
function eq(a, b, name) {
  if (JSON.stringify(a) !== JSON.stringify(b)) { console.error('FAIL ' + name + '\n  got ' + JSON.stringify(a) + '\n  exp ' + JSON.stringify(b)); process.exit(1); }
  n++;
}
const ok = (c, name) => eq(!!c, true, name);

// chatView
eq(I.chatView({ text: 'Hello', lang: 'en' }, 'en'), { text: 'Hello', mtSource: null }, 'en→en');
eq(I.chatView({ text: '안녕', text_en: 'Hi', lang: 'ko' }, 'ko'), { text: '안녕', mtSource: null }, '같은 로케일은 원문');
eq(I.chatView({ text: '안녕', text_en: 'Hi', lang: 'ko' }, 'en'), { text: 'Hi', mtSource: null }, 'en 관전자는 text_en');
eq(I.chatView({ text: '안녕', lang: 'ko' }, 'en'), { text: '안녕', mtSource: '안녕' }, 'en인데 text_en 없음 → 원문 MT');
eq(I.chatView({ text: '안녕하세요 여러분', lang: 'en' }, 'ko'), { text: '안녕하세요 여러분', mtSource: null }, '레거시 한글 오표기 행 안전망');
eq(I.chatView({ text: 'Good morning', lang: 'en' }, 'ko'), { text: 'Good morning', mtSource: 'Good morning' }, '미스: 원문 + mtSource');
eq(I.chatView({ text: '안녕', text_en: 'Hello there', lang: 'ko' }, 'ja'), { text: '안녕', mtSource: 'Hello there' }, '사용자 ko 행을 ja가 볼 때 원천=text_en');
eq(I.chatView({ text: 'Good morning', orig: 'Good morning', lang: 'en' }, 'zh').mtSource, 'Good morning', 'orig 사용');
eq(I.chatView({ text: 'T', orig: 'ORIG', lang: 'en' }, 'ja').mtSource, 'ORIG', 'orig가 text보다 우선');
eq(I.chatView({ text: '', lang: 'en' }, 'ko'), { text: '', mtSource: null }, '빈 문장');
// 캐시 히트: translateText가 fetch 성공해야 채워진다 — fetch 스텁 교체
ctx.fetch = (u, o) => Promise.resolve({ ok: true, json: () => Promise.resolve({ texts: { 'See you soon': '곧 만나요' } }) });
Object.assign(win, { fetch: ctx.fetch });

(async () => {
  const r = await I.translateText('ko', 'See you soon');
  eq(r, '곧 만나요', 'translateText 응답');
  eq(I.chatView({ text: 'See you soon', lang: 'en' }, 'ko'), { text: '곧 만나요', mtSource: null }, '캐시 히트 즉시 번역문');
  eq(I.chatView({ text: '또 봐요', text_en: 'See you soon', lang: 'ko' }, 'ko'), { text: '또 봐요', mtSource: null }, '같은 로케일 우선');

  // isAlreadyKorean 단일 구현
  ok(I.isAlreadyKorean('안녕하세요'), 'isAlreadyKorean 한글');
  ok(!I.isAlreadyKorean('Hello world'), 'isAlreadyKorean 영문');
  ok(!I.isAlreadyKorean('Ara 안녕 hello there friend'), 'isAlreadyKorean 혼합(영문 우세)');

  // resolveParams — bld/name
  const store = I.renderEventKey('ev.store.ok', { bld: 'HUT', item: 'meat', n: 4 }, 'ko', '', (s) => 'MT:' + s);
  ok(store.indexOf('HUT') < 0, 'ko 문장에 원시 HUT 미노출');
  ok(/오두막|움집|집/.test(store) || store.indexOf(I.D['bld.HUT'][0]) >= 0, 'bld 라벨 적용: ' + store);
  const en = I.renderEventKey('ev.store.ok', { bld: 'HUT', item: 'meat', n: 4 }, 'en', '', (s) => s);
  ok(en.indexOf(I.D['bld.HUT'][1]) >= 0, 'en bld 라벨');
  // name 파라미터 — 탬플릿 중 {name} 사용 키 탐색
  const key = Object.keys(win.TF_DATA.EVENT_DICT).find((k) => win.TF_DATA.EVENT_DICT[k][1].indexOf('{name}') >= 0);
  ok(!!key, '{name} 템플릿 존재: ' + key);
  const tm = (s) => 'MT:' + s;
  const before = I.renderEventKey(key, { name: 'Campfire Lamp' }, 'ko', '', tm);
  ok(before.indexOf('Campfire Lamp') >= 0 && before.indexOf('MT:') < 0, '주민 집합 비었을 때 MT 금지(안전)');
  I.setAgentNames(['아라', 'Mira']);
  ok(I.renderEventKey(key, { name: 'Mira' }, 'ko', '', tm).indexOf('MT:') < 0, '주민 이름(영문)은 그대로');
  ok(I.renderEventKey(key, { name: '아라' }, 'ko', '', tm).indexOf('MT:') < 0, '주민 이름(한글)은 그대로');
  ok(I.renderEventKey(key, { name: '새이름' }, 'ko', '', tm).indexOf('MT:') < 0, '한글 전용 값은 이름으로 간주');
  ok(I.renderEventKey(key, { name: 'Campfire Lamp' }, 'ko', '', tm).indexOf('MT:Campfire Lamp') >= 0, '발명품 이름은 mtFn 경유');
  // highlightOf mtSource
  const o = { locale: 'ko', tOr: (k, f) => f, mt: (s) => s, nameOf: () => 'Mira' };
  const free = I.highlightOf({ kind: 'talk', agent_id: 'a', payload: { text: 'An idea about boats' } }, o);
  eq(free && free.mtSource, 'An idea about boats', '키 없는 자유문 토스트는 mtSource');
  const keyed = I.highlightOf({ kind: 'result', agent_id: 'a', payload: { action: 'store', ok: true, key: 'ev.store.ok', params: { bld: 'HUT', item: 'meat', n: 4 }, text: 'Stored 4 meat in the HUT' } }, o);
  eq(keyed && keyed.mtSource, null, '템플릿 토스트는 mtSource 없음(재번역 금지)');
  console.log('OK ' + n + ' checks');
})();
