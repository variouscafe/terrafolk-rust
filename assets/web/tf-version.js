// 게임 하단 버전 표시 — Cargo.toml [package] version이 wasm 컴파일 타임
// 상수(env!("CARGO_PKG_VERSION"))로 주입돼 부팅 직후 terrafolk.setVersion()
// 으로 전달된다. 로컬·배포 동일하게 단일 진실원(Cargo.toml)을 표시한다는
// 점에서 tf-update.js(배포 감시 — meta=wasm 해시)와 축이 다르다.
(function () {
  function render(v) {
    var el = document.getElementById('tf-version');
    if (!el) {
      el = document.createElement('div');
      el.id = 'tf-version';
      // 좌하단 고정(미니맵은 좌상단 top:46px) — 게임 크롬 최소화: 반투명
      // 소자 11px, 포인터 이벤트 통과.
      el.style.cssText = 'position:fixed;left:12px;bottom:10px;z-index:26;' +
        'font:11px/1 ui-sans-serif,system-ui,sans-serif;color:rgba(248,245,239,.55);' +
        'pointer-events:none;user-select:none;letter-spacing:.02em;';
      (document.body || document.documentElement).appendChild(el);
    }
    el.textContent = 'v' + v;
  }
  window.terrafolk = window.terrafolk || {};
  window.terrafolk.setVersion = function (v) {
    if (document.body) render(v);
    else document.addEventListener('DOMContentLoaded', function () { render(v); });
  };
})();
