#!/usr/bin/env node
/** FE(../terrafolk) 정본 → 네이티브 i18n 데이터 생성(src/ui/i18n_data.rs).
 *
 * 원천(수동 편집 금지 — 이 스크립트로만 재생성):
 *   src/lib/i18n/events.gen.ts  → EVENT_DICT(ev.*·goal.* 템플릿, 4로케일)
 *   src/lib/i18n/vocab.gen.ts   → VOCAB(item·sp·dir·invcat·era·bldstage·kind·bld)
 *   src/lib/i18n.ts             → UI_D(act·season·needs·hist·rel·panel.log.tab·tip·menu.log)
 *
 * 로케일은 ko 우선 + en 폴백 2개만 담는다(네이티브 UI는 한국어 고정 —
 * 4로케일 전개는 설정 언어 기능 추가 시 이 스크립트 확장).
 * 조사 토큰({x이가} 등)은 ko 템플릿에 그대로 포함 — 런타임(src/ui/i18n.rs)이 처리.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const FE = join(import.meta.dirname, '..', '..', 'terrafolk');
const OUT = join(import.meta.dirname, '..', 'src', 'ui', 'i18n_data.rs');

function extract(src, name) {
  const re = new RegExp(`(?:export\\s+)?const\\s+${name}\\b[^=]*=`);
  const m = re.exec(src);
  if (!m) throw new Error(`원천에서 ${name}을(를) 찾을 수 없음`);
  let i = m.index + m[0].length;
  while (src[i] === ' ') i++;
  const open = src[i];
  const close = open === '{' ? '}' : ']';
  let depth = 0, str = null, esc = false;
  let j = i;
  for (; j < src.length; j++) {
    const c = src[j];
    if (str) {
      if (esc) esc = false;
      else if (c === '\\') esc = true;
      else if (c === str) str = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') { str = c; continue; }
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) { j++; break; }
    }
  }
  let lit = src.slice(i, j).replace(/\s+as\s+const\s*;?$/, '').replace(/;$/, '');
  return Function(`"use strict"; return (${lit});`)();
}

const events = readFileSync(join(FE, 'src/lib/i18n/events.gen.ts'), 'utf8');
const vocab = readFileSync(join(FE, 'src/lib/i18n/vocab.gen.ts'), 'utf8');
const ITEM_WT = extract(vocab, 'ITEM_WT');
const WORN_ITEMS = [...new Set(extract(vocab, 'WORN_ITEMS'))];
const INVCAT_GROUPS = extract(vocab, 'INVCAT_GROUPS');
const i18n = readFileSync(join(FE, 'src/lib/i18n.ts'), 'utf8');

const EVENT_DICT = extract(events, 'EVENT_DICT');
const VOCAB_ALL = extract(vocab, 'VOCAB_DICT');
const D = extract(i18n, 'D');

/** VOCAB에서 렌더 파이프라인(resolveParams·건물 툴팁)이 참조하는 접두사만. */
const VOCAB_PREFIX = ['item.', 'sp.', 'dir.', 'invcat.', 'era.', 'invgroup.', 'bldstage.', 'kind.', 'bld.'];
const UI_PREFIX = [
  'act.', 'season.', 'needs.', 'hist.', 'rel.',
  'panel.log.', 'tip.grave', 'tip.hut.', 'tip.relic', 'menu.log',
  'civ.', 'inv.', 'relic.', 'invgroup.', 'era.',
];

const pick = (obj, prefixes) =>
  Object.keys(obj)
    .filter((k) => prefixes.some((p) => k.startsWith(p)))
    .sort()
    .map((k) => {
      const t = obj[k];
      return [k, String(t[0] ?? ''), String(t[1] ?? '')];
    });

const evRows = Object.keys(EVENT_DICT).sort().map((k) => {
  const t = EVENT_DICT[k];
  return [k, String(t[0] ?? ''), String(t[1] ?? '')];
});
const vocabRows = pick(VOCAB_ALL, VOCAB_PREFIX);
// bld.* 라벨은 D에도 있을 수 있다 — VOCAB이 잡지 못한 것만 보충.
const vocabKeys = new Set(vocabRows.map((r) => r[0]));
const uiRows = [
  ...pick(D, UI_PREFIX),
  ...pick(VOCAB_ALL, ['bld.']).filter((r) => !vocabKeys.has(r[0])),
];

const wtRows = Object.keys(ITEM_WT).sort().map((k) => [k, ITEM_WT[k]]);
const wornRows = WORN_ITEMS.map((k) => [k, 0]);
const catRows = Object.keys(INVCAT_GROUPS).sort().map((k) => [k, INVCAT_GROUPS[k]]);

function rs(s) {
  return '"' + s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n') + '"';
}
const rows = (name, data) =>
  `pub static ${name}: &[(&str, &str, &str)] = &[\n` +
  data.map(([k, ko, en]) => `    (${rs(k)}, ${rs(ko)}, ${rs(en)}),`).join('\n') +
  '\n];\n';
const kvRows = (name, data, val, ty = '(&str, &str)') =>
  `pub static ${name}: &[${ty}] = &[\n` +
  data.map(([k, v]) => `    (${rs(k)}, ${val(v)}),`).join('\n') +
  '\n];\n';

const header = `// 자동 생성 — tools/gen-native-i18n.mjs (수동 편집 금지). 원천: ../terrafolk (gen ${new Date().toISOString()})
//! 이벤트 템플릿(§17.44)·어휘·UI 문구 — 네이티브 렌더(src/ui/i18n.rs)의 데이터.
//! (key, ko, en) — ko 우선, 빈값이면 en 폴백. 조사 토큰은 런타임 처리.

`;
writeFileSync(
  OUT,
  header + rows('EVENT_DICT', evRows) + '\n' + rows('VOCAB', vocabRows) + '\n' + rows('UI_D', uiRows) +
    '\n' + kvRows('ITEM_WT', wtRows, (v) => (Number.isInteger(v) ? v.toFixed(1) : String(v)), '(&str, f32)') +
    '\n' + kvRows('WORN_ITEMS', wornRows, () => '""') +
    '\n' + kvRows('INVCAT_GROUPS', catRows, (v) => rs(v)),
);

// 원천 대조 어설션 — 누락이면 gen이 실패해야 한다(FE 키 추가 시 재생성 강제).
if (evRows.length !== Object.keys(EVENT_DICT).length) throw new Error('EVENT_DICT 추출 누락');
console.log(
  `[gen-native-i18n] i18n_data.rs — 이벤트 ${evRows.length} · 어휘 ${vocabRows.length} · UI ${uiRows.length} · 무게 ${wtRows.length} · 착용 ${wornRows.length} · 범주 ${catRows.length}`,
);
