// Terrafolk REST 계층 — tf-ui.js에서 기능 분리(500줄 규칙·기능별 분리,
// 2026-11-03). 인증 fetch 래퍼와 모든 백엔드 호출(이벤트·채팅 백로그, 관계
// 목록, 선택 상세, /me, 참여, look 저장, 로그아웃)·공지 바(notify)·내 주민
// id 전달(pushMine — Rust '내 집' 마커 §5 갭2)을 소유한다.
// 클로저 의존은 TF_REST.create(ctx)로 tf-ui.js가 주입한다. 로드 순서:
// tf-ui.js와 무관(정의 시점 의존 없음)이며 index.html 계약상 tf-ui.js보다
// 앞에 둔다(tf-civ 등 패널 모듈과 동일).
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, t = ctx.t, I = ctx.I, schedule = ctx.schedule,
        liveSignature = ctx.liveSignature;

    // 행 id — feed 계층(tf-feed)도 rest.nextRowId로 재사용.
    var rowSeq = 0;
    function nextRowId() {
      rowSeq = (rowSeq + 1) % 10000;
      return Date.now() * 10000 + rowSeq;
    }

    function apiBase() { return (window.TF && window.TF.apiBase) || ''; }
    function authFetch(path, opts) {
      var f = window.terrafolk && window.terrafolk.authFetch;
      if (f) return f(apiBase() + path, opts);
      return fetch(apiBase() + path, opts);
    }
    function hasToken() {
      return !!(window.terrafolk && window.terrafolk.getAuth && window.terrafolk.getAuth().at);
    }

    // ── 공지 바 — 원문 즉시 표시 후 비-en 로케일이면 표시 번역으로 갱신 ──
    function notify(msg) {
      S.notice = String(msg || '');
      schedule();
      if (I.getLocale() !== 'en' && S.notice) {
        I.translateText(I.getLocale(), S.notice).then(function (tr) {
          if (tr !== S.notice) { S.notice = tr; schedule(); }
        }).catch(function () {});
      }
    }

    var backlogSeeded = false;
    /** 백로그(이벤트·채팅) — 부팅 시 1회 시딩. 실패 시 2.5s 간격 3회 재시도. */
    function loadBacklogs() {
      if (backlogSeeded) return;
      var tries = 0;
      var load = function () {
        var okEvents = false, okChat = false;
        authFetch('/api/worldsim/events').then(function (r) { return r.ok ? r.json() : null; })
          .then(function (rows) {
            if (rows) {
              okEvents = true;
              var seeded = rows.slice(-400).map(function (r2) {
                return { id: r2.id, tick: r2.tick, kind: r2.kind, agent_id: r2.agent_id, payload: r2.payload, ts: r2.ts };
              });
              S.events = seeded.concat(S.events).slice(-400);
              schedule();
            }
          }).catch(function () {}).finally(function () { retryCheck(okEvents); });
        authFetch('/api/worldsim/chat').then(function (r) { return r.ok ? r.json() : null; })
          .then(function (rows) {
            if (rows) {
              okChat = true;
              var seeded = rows.slice(-120).map(function (r2) {
                return Object.assign({}, r2, { id: nextRowId(), orig: r2.text });
              });
              S.chat = seeded.concat(S.chat).slice(-120);
              schedule();
            }
          }).catch(function () {}).finally(function () { retryCheck(okChat); });
      };
      var retryCheck = function (ok) {
        if (ok) backlogSeeded = true;
        else if (++tries < 3) setTimeout(function () { if (!backlogSeeded) load(); }, 2500);
      };
      load();
    }

    var relTimer = null;
    function loadRelations() {
      if (relTimer) clearInterval(relTimer);
      var load = function () {
        authFetch('/api/worldsim/relations').then(function (r) { return r.ok ? r.json() : null; })
          .then(function (rows) { if (rows) { S.relations = rows; if (S.popup === 'chars') schedule(); } })
          .catch(function () {});
      };
      load();
      relTimer = setInterval(load, 20000);
    }

    var selTimer = null;
    /** 선택 주민 상세 — 생활기록(공개)·기억(로그인), 15s 갱신. */
    function onSelected() {
      if (selTimer) { clearInterval(selTimer); selTimer = null; }
      if (!S.selected) return;
      var load = function () {
        authFetch('/api/worldsim/agents/' + encodeURIComponent(S.selected) + '/history')
          .then(function (r) { return r.ok ? r.json() : null; })
          .then(function (rows) {
            if (rows) {
              S.history = rows.slice(0, 25).map(function (h) {
                return { ts: h.ts, type: h.type, text: h.text, orig: h.text };
              });
              if (S.popup === 'chars') schedule();
            }
          }).catch(function () {});
        if (S.scope && S.scope.kind === 'user') {
          authFetch('/api/worldsim/agents/' + encodeURIComponent(S.selected) + '/memories')
            .then(function (r) { return r.ok ? r.json() : null; })
            .then(function (rows) {
              if (rows) {
                S.memories = rows.slice(0, 20).map(function (m) {
                  return { id: m.id, kind: m.kind, text: m.text, orig: m.text, ts: m.ts };
                });
                if (S.popup === 'chars') schedule();
              }
            }).catch(function () {});
        }
      };
      load();
      selTimer = setInterval(load, 15000);
    }

    // 내 주민 id — Rust '내 집' 마커(§5 갭2)로 전달. wasm 브리지(tf.setMine)는
    // Rust 부팅 후 생기므로 prime 시 재전송한다(index.html uiPrime → pushMine).
    var mineIds = [];
    function pushMine() {
      var tf = window.terrafolk;
      if (tf && tf.setMine) {
        try { tf.setMine(mineIds); } catch (e) { /* 브리지 미등록 — prime에서 재시도 */ }
      }
    }
    /** 로그아웃 등 — 내 집 마커 즉시 해제. */
    function clearMine() {
      mineIds = [];
      pushMine();
    }

    function loadMine() {
      if (!(S.scope && S.scope.kind === 'user')) return;
      authFetch('/api/worldsim/me').then(function (r) { return r.ok ? r.json() : null; })
        .then(function (mine) {
          if (!mine) return;
          mine.forEach(function (m) {
            var cur = S.live[m.id];
            if (cur) { cur.mine = true; cur.name = m.name; }
          });
          S.liveSig = liveSignature();
          mineIds = mine.map(function (m) { return m.id; });
          pushMine();
          S.mineDetail = mine.map(function (m) {
            return {
              id: m.id, name: m.name, sex: m.sex || 'female', age: m.age || 30,
              parts: m.parts || {}, wears: m.wears || {},
              inventory: m.inventory || {}, clothingInventions: m.clothingInventions || [],
            };
          });
          schedule();
        }).catch(function () {});
    }

    function join() {
      if (S.busy || !S.joinForm.name.trim()) return;
      S.busy = true;
      schedule();
      authFetch('/api/worldsim/join', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: S.joinForm.name.trim() || t('join.name.default'),
          persona: {
            age: Number(S.joinForm.age) || 30,
            sex: S.joinForm.sex,
            persona: S.joinForm.persona.trim() || undefined,
          },
        }),
      }).then(function (r) {
        return r.ok ? r.json() : r.json().then(function (j) { throw new Error(j.message || j.detail || 'join failed'); });
      }).then(function (res) {
        S.busy = false;
        S.joinForm.name = '';
        S.joinForm.persona = '';
        notify(t('notice.join', { name: res.name }));
        loadMine();
        schedule();
      }).catch(function (e) {
        S.busy = false;
        notify((e && e.message) || t('err.generic'));
        schedule();
      });
    }

    // ── 편집 미리보기 브리지(look_edit.rs) ── 카메라 포커스(내 캐릭터 줌 인
    // 팔로우)와 즉시 로컬 look 오버라이드(저장 왕복 전 재스폰).
    function lookFocus(on, agentId) {
      var tf = window.terrafolk || {};
      if (on && agentId && tf.selectAgent) tf.selectAgent(agentId);
      if (tf.setLookFocus) tf.setLookFocus(on);
    }
    function localLookPush(E) {
      var f = window.terrafolk && window.terrafolk.setLocalLook;
      if (f) f(E.agentId, JSON.stringify(E.fullParts()));
    }

    function lookSave(fromClothes) {
      var E = S.lookEdit;
      if (!E || E.busy) return;
      E.busy = true;
      schedule();
      // 옷 슬롯은 wears가 관장 — 수동 parts의 옷 잔존값은 정리해 파생이 우선하게.
      // wears는 에디터 선택(E.wears, 미지정 슬롯=auto 초기화)을 그대로 — 하드코딩
      // auto면 사용자 선택·기존 저장값이 전부 지워진다(FE LookEditor 규약)
      var partsOut = Object.assign({}, E.parts);
      var wearsOut;
      if (fromClothes !== false) {
        // 옷 탭 편집 — 옷 파트는 wears가 관장(소지 파생 우선, FE 규약)
        ['top', 'topColor', 'bottom', 'bottomColor', 'feet', 'feetColor'].forEach(function (k) { delete partsOut[k]; });
        wearsOut = { top: E.wears.top || 'auto', bottom: E.wears.bottom || 'auto', feet: E.wears.feet || 'auto' };
      } else {
        // 헤어·얼굴 편집 — 옷은 건드리지 않는다(기존 wears 그대로 송신)
        wearsOut = Object.assign({}, E.wears);
      }
      authFetch('/api/worldsim/agents/' + encodeURIComponent(E.agentId) + '/look', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ parts: partsOut, wears: wearsOut }),
      }).then(function (r) {
        if (!r.ok) return r.json().then(function (j) { throw new Error(j.message || j.detail || 'save failed'); });
        return r.json();
      }).then(function () {
        // 편집기 유지(2026-10-23) — 슬라이드마다 즉시 저장되므로 닫지 않고
        // 계속 조정하게 한다. 실 프리뷰는 투명 오버레이 너머 3D 세계.
        E.busy = false;
        notify(t('look.saved'));
        loadMine();
        schedule();
      }).catch(function (e) {
        E.busy = false;
        notify((e && e.message) || t('look.err.save'));
        schedule();
      });
    }

    function logout() {
      var f = window.terrafolk && window.terrafolk.logout;
      if (f) Promise.resolve(f()).then(function () {
        S.scope = null;
        S.mineDetail = [];
        Object.keys(S.live).forEach(function (k) { S.live[k].mine = false; });
        schedule();
      });
    }

    return {
      authFetch: authFetch, hasToken: hasToken, notify: notify,
      nextRowId: nextRowId,
      loadBacklogs: loadBacklogs, loadRelations: loadRelations, onSelected: onSelected,
      loadMine: loadMine, pushMine: pushMine, clearMine: clearMine,
      join: join, lookFocus: lookFocus, localLookPush: localLookPush,
      lookSave: lookSave, logout: logout,
    };
  }
  window.TF_REST = { create: create };
})();
