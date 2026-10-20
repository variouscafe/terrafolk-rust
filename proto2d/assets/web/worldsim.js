// proto2d 포크(chat sink 인젤로프·?live=0 스위치 추가; 원본 terrafolk-rust/assets/web/worldsim.js와 나란히 유지)
// Terrafolk 라이브 월드 브리지 — colyseus.js(벤더 UMD) 재사용.
// Rust(wasm)는 window.terrafolk.setSink(fn)으로 콜백을 등록하고
// connect(apiBase, wsBase)로 접속을 요청한다. 상태 변화마다 평면 JSON을
// 스킴 → sink 로 흘린다(패치 주기 1s — 전체 스냅숏 전송으로 충분).
// DOM UI(tf-ui.js — ☰ 메뉴·팝업, ui.js — 채팅 바)에는 상세 프레임
// (욕구·목표·인벤·발명)과 채팅 행을 함께 먹인다.
//
// 인증: OAuth 콜백 fragment(#at=..&rt=..) → localStorage. 401 시 rt로 1회 갱신.
// 접속 실패·끊김은 지수 백오프 재시도 + 탭 복귀·네트워크 회복 즉시 재시도.
(function () {
  'use strict';
  const AUTH_API = 'https://suseona-api.suseona.com';
  const AUTH_KEY = 'tf-rust.auth';
  let sink = null;
  let room = null;
  let retryTimer = null;
  let retry = 0;
  // 접속 진행 중 가드 — boot()의 connect가 티켓 fetch 중일 때 wake()(visibility
  // change·online)가 alive=false를 보고 재접속해 룸이 2개 열리는 경합 방지.
  let connecting = false;
  let alive = false;
  let lastMap = null; // mapJson은 fullStateOnly — 최초 1회만 오므로 캐시
  const names = {}; // agent id → 표시명(채팅 스피커)
  const agentsInfo = {}; // agent id → {name, actType, status} — ui.js 헤더 칩·행 판정
  const chatRowListeners = []; // ui.js — 대화 로그 패널 수신기
  const mtQueue = []; // 표시 계층 번역 대기(§17.51) — ui.translate가 채움(wasm가 이미 중복 제거)
  let mtTimer = null;
  // 발명 아이콘 합성 캐시(§17.36 직역) — key → PNG dataURL
  const iconCache = new Map();

  /** 스펙 → 픽셀 아트 PNG dataURL(레거시 itemIcon.ts 직역: 재색상 → 16px
   *  평균 축소 → 알파 스냅 → 2x nearest). 실패 시 null — 호출부 폴백. */
  function renderIconSync(spec) {
    return new Promise((resolve) => {
      fetch('assets/web/icons/' + spec.tpl + '.svg')
        .then(function (r) { return r.ok ? r.text() : null; })
        .then(function (raw) {
          if (!raw) return resolve(null);
          const body = raw
            .replace(/<path d="M0 0h512v512H0z"\/>/, '')
            .replace(/#fff\b/gi, spec.primary);
          const url = URL.createObjectURL(new Blob([body], { type: 'image/svg+xml' }));
          const img = new Image();
          img.onload = function () {
            URL.revokeObjectURL(url);
            const grid = document.createElement('canvas');
            grid.width = 16; grid.height = 16;
            const g = grid.getContext('2d');
            g.imageSmoothingEnabled = true;
            g.drawImage(img, 0, 0, 16, 16);
            const d = g.getImageData(0, 0, 16, 16);
            for (let i = 3; i < d.data.length; i += 4) {
              d.data[i] = d.data[i] >= 127 ? 255 : 0; // 알파 스냅 — 픽셀 아트 감각
            }
            g.putImageData(d, 0, 0);
            const out = document.createElement('canvas');
            out.width = 32; out.height = 32;
            const o = out.getContext('2d');
            o.imageSmoothingEnabled = false; // 정수배 nearest
            o.drawImage(grid, 0, 0, 32, 32);
            resolve(out.toDataURL('image/png'));
          };
          img.onerror = function () { URL.revokeObjectURL(url); resolve(null); };
          img.src = url;
        })
        .catch(function () { resolve(null); });
    });
  }

  async function iconDataUrl(key, spec) {
    let p = iconCache.get(key);
    if (p === undefined) {
      p = renderIconSync(spec);
      iconCache.set(key, p);
    }
    return p;
  }

  // ── 인증(FE api/client.ts 축소 포트 — body rt 레인 1회 갱신) ──
  function readAuth() {
    try {
      return JSON.parse(localStorage.getItem(AUTH_KEY) || 'null');
    } catch (e) { return null; }
  }
  function writeAuth(a) {
    try {
      if (a) localStorage.setItem(AUTH_KEY, JSON.stringify(a));
      else localStorage.removeItem(AUTH_KEY);
    } catch (e) { /* 저장소 차단 */ }
  }
  // 토큰 fragment 핸드셰이크 — OAuth 콜백이 #at=..&rt=..&provider=..로 돌아온다
  (function captureHash() {
    const hash = location.hash.slice(1);
    if (!hash) return;
    const params = new URLSearchParams(hash);
    const at = params.get('at');
    const rt = params.get('rt');
    const provider = params.get('provider') || undefined;
    if (at && rt) {
      writeAuth({ at: at, rt: rt, provider: provider });
      history.replaceState(null, '', location.pathname + location.search);
    }
  })();

  let refreshing = null;
  function refreshAccessToken() {
    if (!refreshing) {
      const prev = readAuth();
      refreshing = fetch(AUTH_API + '/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken: prev ? prev.rt : '' }),
        credentials: 'include',
      }).then(function (r) {
        if (!r.ok) throw new Error('refresh ' + r.status);
        return r.json();
      }).then(function (data) {
        if (!data.accessToken) throw new Error('no accessToken');
        const cur = readAuth() || {};
        writeAuth({
          at: data.accessToken,
          rt: data.refreshToken != null ? data.refreshToken : cur.rt,
          provider: cur.provider,
        });
        return true;
      }).catch(function () {
        writeAuth(null);
        return false;
      }).finally(function () { refreshing = null; });
    }
    return refreshing;
  }
  /** 인증 fetch — Bearer 부착 + 401 시 1회 갱신 재시도. 미로그인이면 그냥 fetch. */
  function authFetch(url, opts) {
    opts = opts || {};
    const a = readAuth();
    if (a && a.at) {
      opts.headers = Object.assign({}, opts.headers, { Authorization: 'Bearer ' + a.at });
    }
    return fetch(url, opts).then(function (r) {
      if (r.status !== 401 || !a) return r;
      return refreshAccessToken().then(function (ok) {
        if (!ok) return r;
        const a2 = readAuth();
        opts.headers = Object.assign({}, opts.headers, { Authorization: 'Bearer ' + a2.at });
        return fetch(url, opts);
      });
    });
  }

  async function logoutEverywhere() {
    const a = readAuth();
    writeAuth(null);
    try {
      if (a && a.rt) {
        await fetch(AUTH_API + '/auth/logout', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ refreshToken: a.rt }),
          credentials: 'include',
        });
      }
    } catch (e) { /* 클라이언트 세션은 정리 완료 */ }
  }

  // ── HUD — 접속 상태 배지(관전 채팅·이벤트의 화면 표현은 말풍선·토스트가 담당)
  let hud = null;
  let badge = null;

  function ensureHud() {
    if (hud) return;
    // proto2d — DOM 배지 대신 egui 상단바(● LIVE)가 접속 상태 정본.
    // 콘솔·TF_UI 훅은 유지하되 화면 크롬은 만들지 않는다.
    hud = document.createElement('div');
    hud.style.display = 'none';
    document.body.appendChild(hud);
    badge = document.createElement('div');
    hud.appendChild(badge);
    return;
  }
  function setStatus(live) {
    ensureHud();
    badge.textContent = live ? '● LIVE · terrafolk' : '○ DEMO — 서버 재접속';
    badge.style.background = live ? 'rgba(22,101,52,.78)' : 'rgba(120,72,20,.72)';
    if (window.TF_UI) window.TF_UI.setNet(live);
  }

  let lastFrameData = null; // E2E 관측(ui.lastFrame) — 상태 원본 보존
  function snapshot(st) {
    lastFrameData = st;
    const agents = [];
    st.agents.forEach(function (a, id) {
      names[id] = a.name;
      agentsInfo[id] = { name: a.name, actType: a.actType, status: a.status };
      agents.push({
        id: id, name: a.name, kind: a.kind,
        x: a.x, y: a.y, facing: a.facing,
        status: a.status, actType: a.actType,
        actProgress: a.actProgress != null ? a.actProgress : 0,
        look: a.lookJson,
        // UI 상세(팝업용) — Rust LiveAgent는 미지정 필드를 무시한다
        needs: { hunger: a.hunger, thirst: a.thirst, fatigue: a.fatigue, warmth: a.warmth },
        goalText: a.goalText || '', goalKey: a.goalKey || '',
        inventory: a.invJson,
      });
    });
    const resources = [];
    st.resources.forEach(function (r) {
      resources.push({
        id: r.id, kind: r.kind, species: r.species,
        x: r.x, y: r.y, state: r.stateJson,
      });
    });
    if (typeof st.mapJson === 'string' && st.mapJson) {
      lastMap = st.mapJson;
      // 미니맵 베이크용 1회 노출 — tf-world-ui.js가 읽고 파싱한다(매초 전파 방지)
      if (!window.TF_MAP_JSON) window.TF_MAP_JSON = st.mapJson;
    }
    // 발명품 레지스트리(§17.30) — 문명 팝업·인벤 칩 아이콘의 원천
    let inventions = [];
    try {
      inventions = st.inventionsJson ? JSON.parse(st.inventionsJson) : [];
    } catch (e) { /* inventionsJson 파싱 실패 — 빈 목록 */ }
    if (sink) {
      try {
        sink(JSON.stringify({
          tick: st.tick, day: st.day, hour: st.hour, minute: st.minute,
          season: st.season, weather: st.weather,
          sunrise: st.sunrise, sunset: st.sunset,
          map: lastMap,
          // 발명 레지스트리 원문(§17.30) — wasm LiveFrame.inventions는 문자열
          // 필드다(배열 아님 — 2026-10-21 문명 패널 '발명 없음' 원인).
          // 위 inventions 배열 계산은 TF_UI(DOM) 전용이므로 여기선 원문 그대로.
          inventions: st.inventionsJson || null,
          agents: agents, resources: resources,
        }));
      } catch (e) { console.error('[worldsim] sink 오류', e); }
    }
    if (window.TF_UI) {
      window.TF_UI.feed.frame({
        day: st.day, hour: st.hour, minute: st.minute, season: st.season,
        weather: st.weather, sunrise: st.sunrise, sunset: st.sunset,
        agents: agents,
        resources: resources.map(function (r) {
          let state = {};
          try { state = r.state ? JSON.parse(r.state) : {}; } catch (e) { /* pass */ }
          return { id: r.id, kind: r.kind, species: r.species, x: r.x, y: r.y, state: state };
        }),
        inventions: inventions,
      });
    }
  }

  function autoBases() {
    const local = /^(localhost|127\.0\.0\.1)$/.test(location.hostname);
    return local
      ? { api: 'http://127.0.0.1:8892', ws: 'http://127.0.0.1:8893' }
      : { api: 'https://tf-api.suseona.com', ws: 'https://tf-world.suseona.com' };
  }

  // ── 접속(지수 백오프 + 탭 복귀 즉시 재시도, docs §17.29) ──
  function scheduleRetry() {
    alive = false;
    connecting = false;
    room = null;
    retry += 1;
    const wait = Math.min(3000 * 2 ** (retry - 1), 30000);
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = setTimeout(function () { void connect(); }, wait);
  }

  // ── REST 관전 폴러(egui 패리티) — 관계 20s·경제 30s·생활기록 15s(선택 시)·
  // 백로그(채팅·이벤트) 1회. 네이티브 http_poll과 같은 주기·같은 인젤로프.
  let pollStarted = false;
  let watchAgent = null; // Rust가 ui.setWatchAgent으로 지정(선택 주민)

  // sink(Rust) 등록 전 도착한 인벤로프 버퍼 — 느린 wasm 부트에서 백로그
  // 1회 시드가 sink=null에 유실되는 사고 방지(스냅숏과 달리 재전송이 없다).
  const pendingEnvelopes = [];
  function sinkEnvelope(env) {
    const text = JSON.stringify(env);
    if (sink) {
      try { sink(text); } catch (err) { pendingEnvelopes.push(text); }
    } else {
      pendingEnvelopes.push(text);
    }
  }

  async function pollOnce(path) {
    const r = await fetch(path);
    if (!r.ok) throw new Error(path + ' ' + r.status);
    return r.json();
  }

  function startRestPolling(apiBase) {
    if (pollStarted) return;
    pollStarted = true;
    // 백로그 시드 1회 — 채팅 30행(시계순)·이벤트 200행(원시 행 → event 인젤로프)
    (async function () {
      try {
        const rows = await pollOnce(apiBase + '/api/worldsim/chat?limit=30');
        sinkEnvelope({ tf: 'chatbacklog', rows: (rows || []).slice(0, 30) });
      } catch (e) { console.info('[worldsim] 채팅 백로그 스킵', String(e)); }
      try {
        const rows = await pollOnce(apiBase + '/api/worldsim/events?limit=200');
        for (const row of (rows || []).slice(0, 200).reverse()) {
          sinkEnvelope({ tf: 'event', row: row });
        }
      } catch (e) { console.info('[worldsim] 이벤트 백로그 스킵', String(e)); }
    })();
    let t0 = 0, t1 = 0, t2 = 0; // 마지막 성공 시각(0 = 즉시 1회)
    setInterval(function () {
      if (!alive) return;
      const now = Date.now();
      if (now - t0 >= 20000) {
        t0 = now;
        pollOnce(apiBase + '/api/worldsim/relations')
          .then(function (rows) { sinkEnvelope({ tf: 'relations', rows: rows || [] }); })
          .catch(function () { /* 관전 스코프 대기 */ });
      }
      if (now - t1 >= 30000) {
        t1 = now;
        pollOnce(apiBase + '/api/worldsim/economy')
          .then(function (data) { sinkEnvelope({ tf: 'economy', data: data }); })
          .catch(function () { /* 401 등 — 서버 미개방 */ });
      }
      if (watchAgent && now - t2 >= 15000) {
        t2 = now;
        pollOnce(apiBase + '/api/worldsim/agents/' + encodeURIComponent(watchAgent) + '/history')
          .then(function (rows) { sinkEnvelope({ tf: 'history', rows: rows || [] }); })
          .catch(function () { /* 선택 해제·권한 */ });
      }
    }, 5000);
  }

  async function connect(apiBase, wsBase) {
    if (!apiBase || !wsBase) {
      const b = autoBases();
      apiBase = b.api; wsBase = b.ws;
    }
    // tf-i18n(mt 번역)·ui.js(백로그)가 쓰는 공용 베이스
    window.TF = window.TF || {};
    window.TF.apiBase = apiBase;
    window.TF.wsBase = wsBase;
    if (!window.Colyseus || !window.Colyseus.Client) {
      console.warn('[worldsim] colyseus.js 미로딩 — 3초 후 재시도');
      retryTimer = setTimeout(function () { connect(apiBase, wsBase); }, 3000);
      return;
    }
    try {
      if (connecting || alive) return; // 진행 중/연결됨 — 중복 접속 차단
      connecting = true;
      clearTimeout(retryTimer);
      // 티켓 — 로그인이면 인증 티켓(스코프 user), 아니면 관전 티켓(비로그인 게이트 예외).
      // 토큰 만료·갱신 실패는 관전으로 강등(세션은 authFetch가 정리)
      let ticket = null;
      const auth = readAuth();
      if (auth && auth.at) {
        const r = await authFetch(apiBase + '/api/worldsim/ticket');
        if (r.ok) ticket = (await r.json()).ticket;
      }
      if (!ticket) {
        const tRes = await fetch(apiBase + '/api/worldsim/spectate-ticket');
        if (!tRes.ok) throw new Error('티켓 HTTP ' + tRes.status);
        ticket = (await tRes.json()).ticket;
      }

      const client = new window.Colyseus.Client(wsBase);
      client.auth.token = ticket;
      room = await client.joinOrCreate('world');
      console.info('[worldsim] 룸 접속 완료');
      ui.base = apiBase; // 백로그 fetch 등 ui.js가 직접 쓰는 API 베이스
      setStatus(true);
      alive = true;
      connecting = false;
      retry = 0;

      room.onStateChange(function (st) {
        try {
          snapshot(st);
        } catch (e) { console.error('[worldsim] 스냅숏 오류', e); }
      });
      // 스코프 — 룸 hello 메시지({kind:'user'|'spectator', email?})
      room.onMessage('hello', function (m) {
        if (window.TF_UI) window.TF_UI.feed.scope(m || { kind: 'spectator' });
      });
      // 채팅 — 발화 원문 우선(§17.55). 화면 표현은 3D 씬 말풍선(tf-world-ui.js)
      // 와 대화 로그(ui.js)가 담당 — FE와 동일(워드 HUD 줄은 폐지, §17.44 포트).
      room.onMessage('chat', function (m) {
        if (sink) { try { sink(JSON.stringify({ tf: 'chat', row: m })); } catch (err) { console.warn('[worldsim] chat sink 오류', err); } }
        for (const fn of chatRowListeners) {
          try { fn(m); } catch (e) { console.error('[worldsim] ui 수신기 오류', e); }
        }
        if (window.TF_UI) window.TF_UI.feed.chat(m);
      });
      // 이벤트 — 하이라이트 토스트(tf-world-ui.js)·기록 팝업이 소비.
      room.onMessage('event', function (e) {
        if (sink) { try { sink(JSON.stringify({ tf: 'event', row: e })); } catch (err) { console.warn('[worldsim] event sink 오류', err); } }
        if (window.TF_UI) window.TF_UI.feed.event(e);
      });
      room.onLeave(function () {
        console.warn('[worldsim] 퇴장 — 재접속');
        setStatus(false);
        scheduleRetry();
      });
      startRestPolling(apiBase);
      room.onError(function (code, msg) {
        console.warn('[worldsim] 룸 오류', code, msg);
      });
    } catch (e) {
      console.warn('[worldsim] 접속 실패(재시도):', String(e));
      setStatus(false);
      scheduleRetry();
    }
  }

  // 탭 복귀·네트워크 회복 — 끊긴 상태면 즉시 재접속
  function wake() {
    if (alive || document.visibilityState !== 'visible') return;
    if (retryTimer) clearTimeout(retryTimer);
    retryTimer = setTimeout(function () { void connect(); }, 300);
  }
  document.addEventListener('visibilitychange', wake);
  window.addEventListener('online', wake);

  // UI 브리지 — assets/web/ui.js(채팅 바·대화 로그 DOM)가 소비.
  // base는 룸 접속 확정 후 세팅되고, names는 스냅숏마다 갱신되는 live 객체다.
  const ui = {
    base: null,
    names: names,
    live: function () { return !!room; },
    logOpenFlag: false,
    sendChat: function (agentId, text, lang) {
      if (!room) return false;
      room.send('chat', { agentId: agentId, text: text, lang: lang || 'en' });
      return true;
    },
    // 선택 주민 지정 — REST 생활기록 폴러 대상(egui 패널에서 호출)
    setWatchAgent: function (id) { watchAgent = id || null; },
    onChatRow: function (fn) { chatRowListeners.push(fn); },
    agentInfo: function (id) { return agentsInfo[id] || null; },
    // E2E 관측 — 최근 스냅숏 원본(좌표·actType·욕구). 관전 데이터만이라 공개 무해.
    lastFrame: function () { return lastFrameData; },
    // 발명 아이콘 합성(§17.36) — wasm가 [{key,tpl,primary,accent}] JSON로 요청.
    // 결과는 {tf:'icon', items:[{key,png}]} 인벨로프로(png=dataURL base64 뒤).
    renderIcons: function (reqsJson) {
      let reqs = [];
      try { reqs = JSON.parse(reqsJson); } catch (e) { return; }
      Promise.all(reqs.map(async function (r) {
        const png = await iconDataUrl(r.key, r);
        return { key: r.key, png: png ? png.slice('data:image/png;base64,'.length) : '' };
      })).then(function (items) {
        sinkEnvelope({ tf: 'icon', items: items });
      });
    },
    // E2E 패널 구동 큐 — wasm Update가 드레인(ui/mod.rs debug_panel_cmds).
    // 'civ'(토글)·'civ:1'·'civ:0'. 부팅 전 push도 안전(드레인은 등록 후).
    panelCmds: [],
    // 표시 계층 번역(§17.51 직역) — wasm가 JSON 배열로 요청. 디바운스 배치로
    // POST /worldsim/translate → {tf:'mt', texts} 인젤로프로 반환. DB 공유
    // 캐시라 반복 문장은 즉시 히트. 실패 시 응답 없음 — wasm는 원문 유지.
    translate: function (textsJson, target) {
      let texts = [];
      try { texts = JSON.parse(textsJson); } catch (e) { return; }
      if (!Array.isArray(texts) || !texts.length) return;
      for (const t of texts) {
        if (t) mtQueue.push(t);
      }
      if (mtTimer) return;
      mtTimer = setTimeout(function mtFlush() {
        mtTimer = null;
        const batch = mtQueue.splice(0, 40);
        if (!batch.length) { if (mtQueue.length) mtTimer = setTimeout(mtFlush, 350); return; }
        const uniq = Array.from(new Set(batch));
        const base = (window.TF && window.TF.apiBase) || autoBases().api;
        fetch(base + '/api/worldsim/translate', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ texts: uniq, target: target || 'ko' }),
        }).then(function (r) { return r.ok ? r.json() : null; })
          .then(function (data) {
            if (data && data.texts) sinkEnvelope({ tf: 'mt', texts: data.texts });
          })
          .catch(function () { /* 원문 유지 — 표시 안 막힘 */ });
        if (mtQueue.length && !mtTimer) mtTimer = setTimeout(mtFlush, 350);
      }, 350);
    },
  };

  window.terrafolk = {
    setSink: function (fn) {
      sink = fn;
      // 등록 전 쌓인 인벤로프 선행 전송(백로그·폴 1회분 보존)
      while (pendingEnvelopes.length && sink) {
        try { sink(pendingEnvelopes.shift()); } catch (err) { break; }
      }
    },
    connect: connect,
    ui: ui,
    // Rust 3D 씬 빈 땅 클릭 → 유물 카드 판정(tf-ui). pick.rs가 호출.
    uiPick: function (x, y) {
      if (window.TF_UI) window.TF_UI.pickAt(x, y);
    },
    authFetch: authFetch,
    getAuth: function () { return readAuth() || {}; },
    logout: logoutEverywhere,
  };

  // 부팅 — 접속 + 백로그 시딩(기록 팝업을 열면 즉시 최근 목록)
  function boot() {
    if (/live=0/.test(location.search)) return; // proto2d 데모 강제 스위치
    void connect();
    if (window.TF_UI) window.TF_UI.seedBacklogs();
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
})();
