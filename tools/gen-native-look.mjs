#!/usr/bin/env node
/** 웹 정본(assets/web) → 네이티브 외형 편집 데이터 생성(src/ui/look_data.rs).
 *
 * 원천(수동 편집 금지 — 이 스크립트로만 재생성):
 *   assets/web/tf-data.js  → LPC RULES·POOL_ITEMS(옵션 목록의 가중 원본)
 *   assets/web/tf-look.js  → lookPartOptions·SWATCH(버킷별 옵션·스와치 hex)
 *   assets/web/tf-ui.js    → KO(스타일명 한글표)·CLOTHES(옷 카탈로그)
 *
 * tf-look.js를 vm에서 실실행해 lookPartOptions 결과를 그대로 담는다 —
 * 규칙 해석(byBt 필터 등)의 이중 구현을 만들지 않는다(패리티 포팅 규칙).
 * 산출물은 네이티브 외형 편집기(src/ui/look.rs)만 소비.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';

const HERE = import.meta.dirname;
const OUT = join(HERE, '..', 'src', 'ui', 'look_data.rs');

// ── tf-data.js + tf-look.js를 vm 실행(옵션 함수는 DOM 무참조) ──
const ctx = { window: {}, document: undefined, URL, Promise, Map, Object };
vm.createContext(ctx);
vm.runInContext(readFileSync(join(HERE, '..', 'assets/web/tf-data.js'), 'utf8'), ctx);
vm.runInContext(readFileSync(join(HERE, '..', 'assets/web/tf-look.js'), 'utf8'), ctx);
const TF_LOOK = ctx.window.TF_LOOK;
if (!TF_LOOK) throw new Error('tf-look.js 실행 실패 — TF_LOOK 미노출');

// ── tf-ui.js에서 KO·CLOTHES 리터럴 추출(괄호 스캔) ──
function scanLiteral(src, anchor) {
  const m = new RegExp(anchor).exec(src);
  if (!m) throw new Error(`원천에서 ${anchor}을(를) 찾을 수 없음`);
  let i = m.index + m[0].length;
  while (src[i] === ' ') i++;
  const open = src[i];
  const close = open === '{' ? '}' : ']';
  let depth = 0, str = null, esc = false, j = i;
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
    else if (c === close && --depth === 0) { j++; break; }
  }
  return Function(`"use strict"; return (${src.slice(i, j)});`)();
}
const tfUi = readFileSync(join(HERE, '..', 'assets/web/tf-ui.js'), 'utf8');
const KO = scanLiteral(tfUi, 'var KO\\s*=\\s*');
const CLOTHES = scanLiteral(tfUi, 'CLOTHES:\\s*');

// ── 옵션 목록 — 성별×연령 버킷 전개(lookPartOptions에 나이 대표값 전달) ──
const AGES = { teen: 15, adult: 40, senior: 70 };
const opts = {};
for (const g of ['male', 'female']) {
  for (const ac of Object.keys(AGES)) {
    const o = TF_LOOK.lookPartOptions(g, AGES[ac]);
    opts[`${g[0]}_${ac}`] = o;
  }
}
// skins·eyes·beards(남성)는 버킷 불변 — 임의 버킷에서 1회 채취·검증
const flat = opts.m_adult;
for (const k of ['m_teen', 'm_senior']) {
  if (JSON.stringify(opts[k].beards) !== JSON.stringify(flat.beards)) {
    throw new Error(`beards가 버킷별로 다름 — 평탄화 가정 위반(${k})`);
  }
}

// ── 스와치 hex — 옵션 전 키 커버 어설션(누락 색은 에디터 점선 누락 원인) ──
const kinds = { skin: flat.skins, eye: flat.eyes, hairColor: null };
const hairAll = new Set(
  Object.keys(AGES).flatMap((ac) => opts[`m_${ac}`].hairColors),
);
kinds.hairColor = [...hairAll];
const swatchRows = [];
for (const [kind, keys] of Object.entries(kinds)) {
  for (const k of keys) {
    const hex = TF_LOOK.swatch(kind, k);
    // 머리색의 teen·senior 전용색(sandy 등)은 웹 SWATCH에 hex가 없어
    // 웹 편집기도 점을 그리지 않는다(tf-ui swatchRowHtml `if (!hex) return ''`)
    // — 패리티대로 스와치 있는 색만 방출하고 런타임도 이 표로 거른다.
    if (!hex) {
      if (kind !== 'hairColor') throw new Error(`스와치 누락: ${kind}.${k}`);
      continue;
    }
    swatchRows.push([kind, k, hex]);
  }
}

// ── Rust 방출 ──
const rs = (s) => '"' + String(s).replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"';
const list = (xs) => '&[' + xs.map(rs).join(', ') + ']';
const hairColors = Object.keys(AGES)
  .map((ac) => `    (${rs(ac)}, ${list(opts[`f_${ac}`].hairColors)}),`)
  .join('\n');
// hairColors는 성별 무관 확인(연령만 구분)
for (const ac of Object.keys(AGES)) {
  if (JSON.stringify(opts[`m_${ac}`].hairColors) !== JSON.stringify(opts[`f_${ac}`].hairColors)) {
    throw new Error(`hairColors가 성별별로 다름 — 평탄화 가정 위반(${ac})`);
  }
}
const hairs = Object.keys(opts)
  .sort()
  .map((b) => `    (${rs(b)}, ${list(opts[b].hairs)}),`)
  .join('\n');
const glasses = Object.keys(AGES).map((ac) => {
  const g = opts[`m_${ac}`].glasses;
  const gf = opts[`f_${ac}`].glasses;
  if (JSON.stringify(g) !== JSON.stringify(gf)) {
    throw new Error(`glasses가 성별별로 다름 — 평탄화 가정 위반(${ac})`);
  }
  return `    (${rs(ac)}, ${list(g)}),`;
}).join('\n');
const koRows = Object.entries(KO).map(([k, v]) => `    (${rs(k)}, ${rs(v)}),`).join('\n');
const clothesRows = CLOTHES.map((c) => {
  const look = Object.entries(c.look).map(([k, v]) => `(${rs(k)}, ${rs(v)})`).join(', ');
  return `    ClothesEntry { item: ${rs(c.item)}, slot: ${rs(c.slot)}, cf: ${Number(c.cf).toFixed(2)}, look: &[${look}] },`;
}).join('\n');

const header = `// 자동 생성 — tools/gen-native-look.mjs (수동 편집 금지). 원천: assets/web (gen ${new Date().toISOString()})
//! 네이티브 외형 편집기(src/ui/look.rs)의 옵션 데이터 — 웹 편집기(tf-ui
//! LookEditor)와 동일 목록(lookPartOptions 실실행 전개). 스와치 hex는 3D
//! 틴트 팔레트(look_tint.rs)와 동일값. 옷 카탈로그는 FE CLOTHES 미러.

/// 피부색 옵션(버킷 불변).
pub static SKINS: &[&str] = ${list(flat.skins)};

/// 눈색 옵션(버킷 불변).
pub static EYES: &[&str] = ${list(flat.eyes)};

/// 머리색 옵션 — (연령 클래스, 목록). 성별 무관.
pub static HAIR_COLORS: &[(&str, &[&str])] = &[
${hairColors}
];

/// 헤어 스타일 옵션 — (버킷 "{m|f}_{teen|adult|senior}", 목록).
pub static HAIRS: &[(&str, &[&str])] = &[
${hairs}
];

/// 수염 옵션(남성 전용 — 여성은 look.rs가 ['none'] 고정).
pub static BEARDS: &[&str] = ${list(flat.beards)};

/// 안경 옵션 — (연령 클래스, 목록). 성별 무관.
pub static GLASSES: &[(&str, &[&str])] = &[
${glasses}
];

/// 색 스와치 — (kind, 키, hex). kind: skin·eye·hairColor.
pub static SWATCH: &[(&str, &str, &str)] = &[
${swatchRows.map(([a, b, c]) => `    (${rs(a)}, ${rs(b)}, ${rs(c)}),`).join('\n')}
];

/// 스타일 id 마지막 세그먼트 → 한국어 라벨(tf-ui prettyPart KO 표).
pub static KO: &[(&str, &str)] = &[
${koRows}
];

/// 옷 카탈로그(FE CLOTHES 미러) — 소지 기반 잠금·auto 최선 cf 선택의 원본.
pub struct ClothesEntry {
    pub item: &'static str,
    pub slot: &'static str,
    pub cf: f32,
    pub look: &'static [(&'static str, &'static str)],
}

pub static CLOTHES: &[ClothesEntry] = &[
${clothesRows}
];
`;
writeFileSync(OUT, header);
console.log(
  `[gen-native-look] look_data.rs — 스킨 ${flat.skins.length} · 눈 ${flat.eyes.length} · 헤어 버킷 ${Object.keys(opts).length} · 수염 ${flat.beards.length} · 안경 3클래스 · 스와치 ${swatchRows.length} · 한글표 ${Object.keys(KO).length} · 옷 ${CLOTHES.length}`,
);
