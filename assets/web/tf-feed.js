// Terrafolk 라이브 프레임 수신 계층 — tf-ui.js에서 기능 분리(500줄 규칙,
// 2026-11-03). worldsim.js가 호출하는 TF_UI.feed.* 구현(주민·자원·발명
// 스냅숏 반영, 이벤트·채팅 적재, 스코프)과 침전 CSS 동기, 빈 땅 클릭 유물
// 카드 판정(pickAt §17.38)을 소유한다. 클로저 의존은 TF_FEED.create(ctx)로
// tf-ui.js가 주입한다. 로드 순서: tf-ui.js보다 앞(패널 모듈과 동일 계약).
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, I = ctx.I, schedule = ctx.schedule, nextRowId = ctx.nextRowId,
        openPopup = ctx.openPopup, loadMine = ctx.loadMine, clearMine = ctx.clearMine;

    // 침전 오버레이 — CSS(.tf-precip) 구동. 3D 파이프라인은 WebGL2 프래그먼트
    // 유니폼 한도(12) 초과로 침전 재질을 만들 수 없어 여기서 처리한다.
    var precipEl = null;
    function syncPrecip(weather) {
      if (!precipEl) precipEl = document.querySelector('.tf-precip');
      if (!precipEl) return;
      var cls = 'tf-precip';
      if (weather === 'rain') cls += ' rain';
      else if (weather === 'storm') cls += ' rain storm';
      else if (weather === 'snow') cls += ' snow';
      if (precipEl.className !== cls) precipEl.className = cls;
    }

    function frame(f) {
      // 시계
      S.clock = {
        day: f.day, hour: f.hour, minute: f.minute, season: f.season,
        weather: f.weather, sunrise: f.sunrise, sunset: f.sunset,
      };
      syncPrecip(f.weather);
      // 주민 — 프레임에 없는 기존 행은 유지(이름 시드), 퇴장 제거
      var seen = {};
      (f.agents || []).forEach(function (a) {
        seen[a.id] = 1;
        var cur = S.live[a.id];
        S.live[a.id] = {
          id: a.id,
          name: a.name || (cur ? cur.name : a.id),
          status: a.status != null ? a.status : (cur ? cur.status : ''),
          actType: a.actType != null ? a.actType : (cur ? cur.actType : ''),
          actProgress: a.actProgress != null ? a.actProgress : (cur ? cur.actProgress : 0),
          needs: a.needs || (cur ? cur.needs : {}),
          inv: a.inventory != null ? I.parseInv(a.inventory) : (cur ? cur.inv : {}),
          goalText: a.goalText != null ? a.goalText : (cur ? cur.goalText : ''),
          look: a.look != null ? a.look : (cur ? cur.look : null),
          kind: a.kind != null ? a.kind : (cur ? cur.kind : ''),
          homeId: a.homeId != null ? a.homeId : (cur ? cur.homeId : null),
          x: a.x != null ? a.x : (cur ? cur.x : 0),
          y: a.y != null ? a.y : (cur ? cur.y : 0),
          mine: cur ? cur.mine : false,
        };
      });
      Object.keys(S.live).forEach(function (k) { if (!seen[k]) delete S.live[k]; });
      // 주민 이름 집합 주입 — resolveParams {name}이 이름을 MT하지 않게(§17.121)
      if (I.setAgentNames) I.setAgentNames(Object.keys(S.live).map(function (k) { return S.live[k].name; }));
      // 자원 — 스냅숏 시드
      S.resMap = {};
      (f.resources || []).forEach(function (r) {
        S.resMap[r.id] = r;
      });
      // 발명
      if (f.inventions) {
        S.inventionsAll = f.inventions;
        var inv = {};
        f.inventions.forEach(function (i) {
          if (i.icon && i.icon.tpl) {
            inv[i.name] = {
              tpl: i.icon.tpl, primary: i.icon.primary, accent: i.icon.accent,
              purpose: i.purpose, inventor: i.inventor, based_on: i.based_on,
            };
          }
        });
        S.inventions = inv;
      }
      // 내 캐릭 확인 — 주민 집합이 바뀌면
      var sig = Object.keys(S.live).sort().join(',');
      if (sig !== S.liveSig) {
        S.liveSig = sig;
        loadMine();
      }
      // 선택 해제는 Rust 단일 소스(tf.onPickAgent(null))에 맡긴다 — 프레임 공백
      // (재접속·드랍)으로 S.live가 잠깐 비어도 카드·추적이 튕기지 않는다.
      // Rust는 AgentGrace 유예 후 실제 철거 시에만 null을 통보한다.
      schedule();
    }
    function event(e) {
      S.events.push(Object.assign({ id: nextRowId(), ts: new Date().toISOString() }, e));
      if (S.events.length > 400) S.events.splice(0, S.events.length - 400);
      // 스폰 이름 시드 — 접속 후 합류한 주민도 이름으로
      if (e.kind === 'spawn' && e.agent_id && e.payload && e.payload.name) {
        var cur = S.live[e.agent_id];
        if (cur && cur.name !== String(e.payload.name)) cur.name = String(e.payload.name);
        if (I.setAgentNames) I.setAgentNames([e.payload.name]);
      }
      if (S.popup === 'log' || S.popup === 'inventions') schedule();
    }
    function chat(c) {
      S.chat.push(Object.assign({ id: nextRowId(), orig: c.text, ts: new Date().toISOString() }, c));
      if (S.chat.length > 120) S.chat.splice(0, S.chat.length - 120);
      if (S.popup === 'log') schedule();
    }
    function scope(sc) {
      S.scope = sc;
      if (sc && sc.kind === 'user') loadMine();
      else clearMine();   // 로그아웃 — 내 집 마커 해제(§5 갭2)
      schedule();
    }

    // ── Rust 픽 브리지 — 빈 땅 클릭 좌표로 유물 카드(§17.38).
    //    주민 픽킹은 Rust pick.rs가직접 하고 tf.onPickAgent로 돌아온다.
    function pickAt(x, y) {
      // 건물·묘지 탭 툴팁(§17.78 — 터치 고정 2.5s) — tf-world-ui가 판정·표시.
      // 유물 카드보다 우선(발판 안 클릭은 집 안내가 우선).
      if (window.TF_WORLD_UI && window.TF_WORLD_UI.showResAt(x, y)) return;
      var bestR = null, bestRd = 4;
      Object.keys(S.resMap).forEach(function (rid) {
        var r = S.resMap[rid];
        if (r.kind !== 'relic') return;
        var d = (r.x + 0.5 - x) * (r.x + 0.5 - x) + (r.y + 0.5 - y) * (r.y + 0.5 - y);
        if (d < bestRd) { bestRd = d; bestR = r; }
      });
      if (bestR) {
        var st = bestR.state || {};
        S.selRelic = {
          name: bestR.species, by: String(st.by || ''),
          purpose: String(st.purpose || ''), basedOn: String(st.based_on || ''),
        };
        openPopup('res');
      }
    }

    return { frame: frame, event: event, chat: chat, scope: scope, pickAt: pickAt };
  }
  // 하이라이트 토스트(우하단) DOM — tf-world-ui가 호출. 상수는 FE 동일(6s 후 제거, 최대 4개).
  var TOAST_LIFE = 6000;
  var TOAST_MAX = 4;
  var toastHost = null;   // 지연 생성 — 스크립트가 body 이전에 로드돼도 안전

  function pushToast(h) {
    if (!toastHost) {
      toastHost = document.createElement('div');
      toastHost.id = 'tf-toasts';
      document.body.appendChild(toastHost);
    }
    var el = document.createElement('div');
    el.className = 'tf-toast';
    var ic = document.createElement('span');
    ic.className = 'ic';
    ic.textContent = h.icon || '✦';
    var body = document.createElement('span');
    if (h.who) {
      var who = document.createElement('span');
      who.className = 'who';
      who.textContent = h.who + ' — ';
      body.appendChild(who);
    }
    var txtNode = document.createTextNode(h.text || '');
    body.appendChild(txtNode);
    // 자유문 토스트 MT 패치 — h.mtSource(키 없는 영문 정본)일 때만.
    (function () {
      var i = window.TF_I18N;
      var loc = i && i.getLocale();
      if (!i || loc === 'en' || !h.mtSource) return;   // 템플릿 문장은 이미 로케일 렌더 — 재번역 금지(§17.121)
      i.translateText(loc, h.mtSource).then(function (tr) {
        if (tr && tr !== h.text) txtNode.textContent = tr;
      }).catch(function () {});
    })();
    el.appendChild(ic);
    el.appendChild(body);
    toastHost.appendChild(el);
    while (toastHost.children.length > TOAST_MAX) toastHost.removeChild(toastHost.firstChild);
    setTimeout(function () { el.remove(); }, TOAST_LIFE);
  }
  window.TF_FEED = { create: create, pushToast: pushToast };
})();
