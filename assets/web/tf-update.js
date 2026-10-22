// 배포 버전 감시 — 낡은 wasm/JS 캐시로 고장나는 사고(2026-10-22 '하늘색만
// 보임' 보고) 예방. 부팅 버전(meta[name=tf-version], deploy 스크립트가
// wasm 해시로 주입)과 /version.json을 비교해 새 배포가 감지되면 배너로
// 안내·클릭 새로고침. 탭이 숨겨진 상태에선 사용자를 방해하지 않게 자동 갱신.
(function () {
  var meta = document.querySelector('meta[name="tf-version"]');
  if (!meta || !window.fetch) return; // 로컬 개발(glue 미적용)은 무동작
  var cur = meta.getAttribute('content');
  var pending = false;

  function check() {
    if (pending) return;
    fetch('version.json?t=' + Date.now(), { cache: 'no-store' })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (j) { if (j && j.v && j.v !== cur) { pending = true; show(); } })
      .catch(function () { /* 네트워크 순단 — 다음 폴링에 재시도 */ });
  }

  function show() {
    if (document.getElementById('tf-update')) return;
    var el = document.createElement('div');
    el.id = 'tf-update';
    el.style.cssText = 'position:fixed;z-index:80;top:calc(2.25rem + 8px);left:50%;transform:translateX(-50%);' +
      'display:flex;align-items:center;gap:.5rem;padding:.5rem .625rem .5rem .75rem;border-radius:.75rem;' +
      'border:1px solid rgba(252,211,77,.45);background:rgba(11,14,20,.94);backdrop-filter:blur(8px);' +
      'box-shadow:0 8px 24px rgba(0,0,0,.45);font:12px/1.4 ui-sans-serif,system-ui,sans-serif;color:#f8f5ef;';
    el.innerHTML = '<span>🔄 새 버전이 배포되었어요</span>' +
      '<button style="border:0;border-radius:.5rem;padding:.3125rem .75rem;background:#f59e0b;color:#1c1917;' +
      'font:600 12px/1 ui-sans-serif,system-ui,sans-serif;cursor:pointer">업데이트</button>' +
      '<button data-x aria-label="닫기" style="border:0;background:none;color:#9aa3b2;cursor:pointer;font-size:13px">✕</button>';
    document.body.appendChild(el);
    el.querySelector('button').onclick = function () { location.reload(); };
    el.querySelector('[data-x]').onclick = function () { el.remove(); };
  }

  // 첫 확인은 부팅 20초 후(프라임 로딩 방해 없게), 이후 5분 주기 + 탭 복귀 시.
  setTimeout(check, 20000);
  setInterval(check, 5 * 60 * 1000);
  document.addEventListener('visibilitychange', function () {
    if (document.hidden && pending) location.reload(); // 보이지 않는 사이 자동 갱신
    else if (!document.hidden) check();
  });
})();
