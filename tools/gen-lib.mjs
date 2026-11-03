/** 코드젠 공용 유틸 — gen-web-data.mjs·gen-native-i18n.mjs가 공유(2026-11-03,
 * 재사용 분리). 소스에서 `const NAME…= <리터럴>` 추출 — 중괄호/대괄호 균형
 * (문자열 인식) 후 Function 생성자로 평가한다. */
export function extract(src, name) {
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
  let lit = src.slice(i, j)
    .replace(/\s+as\s+const\s*;?$/, '')
    .replace(/;$/, '');
  return Function(`"use strict"; return (${lit});`)();
}
