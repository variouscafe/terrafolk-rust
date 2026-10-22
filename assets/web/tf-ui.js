// Terrafolk ☰ 메뉴·중앙 팝업 DOM 오버레이 — FE worldsim 화면(TopBar·메뉴 드롭다운·
// 팝업 6종·크레딧·외형 편집)의 바닐라 JS 포트. 상태는 worldsim.js(룸 접속)가
// TF_UI.feed.*로 밀어주고, 이 모듈이 렌더·REST(선택 상세·참여·외형)·로케일을 소유한다.
// 채팅 바·대화 로그 독은 ui.js 담당 — 선택 단일 소스는 Rust 픽킹
// (tf.selectAgent 요청 → tf.onPickAgent 통보, src/world/pick.rs). 이 파일은 로드
// 순서상 ui.js 뒤라 기존 onPickAgent 핸들러를 chaining으로 이어받는다.
// 로드 순서: tf-data → tf-i18n → tf-look → ui.js → (tf-ui).
(function () {
  'use strict';
  var I = window.TF_I18N;
  var DATA = window.TF_DATA;

  // ═══════════════ 상태 ═══════════════
  var S = {
    menuOpen: false,
    popup: null,            // 'chars'|'res'|'inventions'|'log'|'set'|'account'
    selected: null,
    selRelic: null,
    selInv: null,
    logCat: 'all',
    notice: '',
    clock: {},
    live: {},               // id → {id,name,status,actType,needs,inv,goalText,mine}
    liveSig: '',
    resMap: {},             // id → {id,kind,species,x,y,state}
    events: [],             // 수신 순(오래된→최신), 렌더에서 reverse
    chat: [],
    relations: [],
    memories: [],
    history: [],
    inventionsAll: [],
    inventions: {},         // name → {tpl,primary,accent,purpose,inventor,based_on}
    scope: null,            // {kind:'user'|'spectator', email?}
    mineDetail: [],
    joinForm: { name: '', age: '30', sex: 'female', persona: '' },
    busy: false,
    lookEdit: null,
    creditsOpen: false,
    netLive: false,
  };
  var rowSeq = 0;
  function nextRowId() {
    rowSeq = (rowSeq + 1) % 10000;
    return Date.now() * 10000 + rowSeq;
  }

  // ═══════════════ 렌더 스케줄 ═══════════════
  var raf = 0;
  function schedule() {
    if (raf) return;
    raf = requestAnimationFrame(function () { raf = 0; render(); });
  }
  // rAF 기아 방지 — 무거운 wasm 렌더 루프·백그라운드 탭에서 rAF가 밀릴 때
  // 대기 중 렌더를 250ms 내 강제 플러시(렌더 대기가 없으면 무동작).
  setInterval(function () {
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
      render();
    }
  }, 250);

  function esc(s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }
  // lucide 아이콘(FE와 동일 모양) — stroke 방식, size px
  var ICONS = {
    Menu: '<line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="18" y2="18"/>',
    Users: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    TreePine: '<path d="m17 14 3 3.3a1 1 0 0 1-.7 1.7H4.7a1 1 0 0 1-.7-1.7L7 14h-.3a1 1 0 0 1-.7-1.7L9 9h-.2A1 1 0 0 1 8 7.3L12 3l4 4.3a1 1 0 0 1-.8 1.7H15l3 3.3a1 1 0 0 1-.7 1.7H17Z"/><path d="M12 22v-3"/>',
    Landmark: '<line x1="3" x2="21" y1="22" y2="22"/><line x1="6" x2="6" y1="18" y2="11"/><line x1="10" x2="10" y1="18" y2="11"/><line x1="14" x2="14" y1="18" y2="11"/><line x1="18" x2="18" y1="18" y2="11"/><polygon points="12 2 20 7 4 7"/>',
    ScrollText: '<path d="M15 12h-5"/><path d="M15 8h-5"/><path d="M19 17V5a2 2 0 0 0-2-2H4"/><path d="M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3"/>',
    Settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
    User: '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    LogIn: '<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><polyline points="10 17 15 12 15 12 10 7"/><line x1="15" x2="3" y1="12" y2="12"/>',
    LogOut: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>',
    X: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    MapPin: '<path d="M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0Z"/><circle cx="12" cy="10" r="3"/>',
    ChevronLeft: '<path d="m15 18-6-6 6-6"/>',
    ChevronRight: '<path d="m9 18 6-6-6-6"/>',
    Lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    UserPlus: '<path d="M2 21a8 8 0 0 1 13.292-6"/><circle cx="10" cy="8" r="5"/><path d="M19 16v6"/><path d="M22 19h-6"/>',
  };
  function svg(paths, size) {
    return '<svg viewBox="0 0 24 24" width="' + size + '" height="' + size +
      '" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
      paths + '</svg>';
  }
  function t(k, p) { return I.t(k, p); }
  function tOr(k, fb, p) { return I.tOr(k, fb, p); }
  var mtMap = {};   // "locale|text" → 번역(완료 후 재렌더)
  function mt(text) {
    var locale = I.getLocale();
    if (locale === 'en' || !text) return text;
    var key = locale + '|' + text;
    if (mtMap[key] !== undefined) return mtMap[key];
    I.translateText(locale, text).then(function (tr) {
      if (tr !== text && mtMap[key] === undefined) {
        mtMap[key] = tr;
        schedule();   // 팝업 몸통·히어로 카드 갱신 — 틱을 기다리지 않는다
      }
    }).catch(function () {});
    return text;
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

  // 아이콘 캐시(칩 <img>) — 스펙 → dataURL, 완료 후 재렌더
  var iconUrls = {};
  function chipIconHtml(name, px) {
    px = px || 18;
    if (DATA.ITEM_EMOJI[name]) return '<span>' + DATA.ITEM_EMOJI[name] + '</span>';
    var spec = S.inventions[name];
    if (spec && spec.tpl) {
      var key = name + '|' + px;
      if (iconUrls[key]) {
        return '<img src="' + iconUrls[key] + '" alt="' + esc(name) + '" width="' + px + '" height="' + px +
          '" style="image-rendering:pixelated;border-radius:2px;background:' + spec.accent + '40">';
      }
      window.TF_LOOK.renderItemIcon(spec, 32).then(function (u) {
        if (u && !iconUrls[key]) { iconUrls[key] = u; schedule(); }
      }).catch(function () {});
      return '<span>⚒</span>';
    }
    return '<span>' + (spec ? '⚒' : '🎒') + '</span>';
  }

  // ── 문명 도우미(FE civ.ts 포트) ──
  function resUsable(kind, state, season) {
    if (!state) return true;
    if (kind === 'tree') return Number(state.maturity != null ? state.maturity : 0) >= 2;
    if (kind === 'gold_vein') return Number(state.amount != null ? state.amount : 0) > 0;
    if (kind === 'plant') {
      if (!state.available) return false;
      var seasons = state.seasons;
      if (Array.isArray(seasons) && seasons.length > 0 && season) return seasons.indexOf(season) >= 0;
      return true;
    }
    if (kind === 'animal') return state.alive !== false;
    return true;
  }
  function resStats() {
    var byKind = {};
    Object.keys(S.resMap).forEach(function (rid) {
      var r = S.resMap[rid];
      if (r.kind === 'campfire' || r.kind === 'hut') return;   // 발명 도감으로(§17.38)
      var k = byKind[r.kind];
      if (!k) k = byKind[r.kind] = { kind: r.kind, total: 0, usable: 0, species: {} };
      var u = resUsable(r.kind, r.state, S.clock.season);
      k.total++;
      if (u) k.usable++;
      var s = k.species[r.species];
      if (!s) s = k.species[r.species] = { n: 0, usable: 0 };
      s.n++;
      if (u) s.usable++;
    });
    return Object.keys(byKind).map(function (k) { return byKind[k]; })
      .sort(function (a, b) { return b.total - a.total; });
  }

  function isHumanRow(c) {
    var a = S.live[c.agent_id];
    return String(c.speaker || '').indexOf('@') >= 0 || (a !== undefined && c.speaker !== a.name);
  }
  function isAlreadyKorean(s) {
    var h = (String(s).match(/[가-힣]/g) || []).length;
    var l = (String(s).match(/[A-Za-z]/g) || []).length;
    return h > 0 && h >= l * 3;
  }
  function chatDisplayText(cm) {
    var src = cm.orig != null ? cm.orig : cm.text;
    var lang = cm.lang || 'en';
    var locale = I.getLocale();
    if (locale === lang) return src;
    if (locale === 'en' && cm.text_en) return String(cm.text_en);
    if (locale === 'ko' && isAlreadyKorean(src)) return src;
    return mt(src);
  }

  // ═══════════════ 상단 바 ═══════════════
  function ensureRoot() {
    var root = document.getElementById('tf-ui');
    if (!root) {
      root = document.createElement('div');
      root.id = 'tf-ui';
      document.body.appendChild(root);
    }
    return root;
  }

  function topbarHtml() {
    var clock = S.clock;
    var n = Object.keys(S.live).length;
    var isUser = S.scope && S.scope.kind === 'user';
    var weather = clock.weather
      ? '<span class="tf-chip">' + (I.WEATHER_ICON[clock.weather] || '') + ' ' +
        tOr('weather.' + clock.weather, clock.weather) + '</span>'
      : '';
    var menuItems = [
      ['chars', t('menu.chars'), ICONS.Users],
      ['res', t('menu.resources'), ICONS.TreePine],
      ['inventions', t('menu.civilization'), ICONS.Landmark],
      ['log', t('menu.log'), ICONS.ScrollText],
      ['set', t('menu.settings'), ICONS.Settings],
      ['account', hasToken() ? t('menu.account') : t('menu.login'), hasToken() ? ICONS.User : ICONS.LogIn],
    ].map(function (m) {
      return '<button class="tf-menu-item" data-act="popup" data-key="' + m[0] + '">' +
        svg(m[2], 14) + ' ' + esc(m[1]) + '</button>';
    }).join('');
    return (
      '<div class="tf-topbar">' +
        '<div class="tf-topbar-row">' +
          '<span class="tf-topbar-title">' + esc(t('world.title')) + '</span>' +
          '<span class="tf-clock-wrap">' +
            I.dayDialSvg(clock.hour, clock.minute, clock.sunrise, clock.sunset) +
            '<span class="tf-clock">' + esc(I.clockText(clock)) + '</span>' +
          '</span>' +
          weather +
          '<span class="tf-chip">' + esc(t('world.people', { n: n })) + '</span>' +
          '<span class="tf-chip tf-chip-mode' + (isUser ? ' tf-chip-user' : '') + '">' +
            esc(isUser ? t('world.joined') : t('world.spectate')) + '</span>' +
          '<span class="tf-topbar-spacer"></span>' +
          '<div class="tf-menu-anchor">' +
            '<button class="tf-menu-btn" data-act="menu" title="' + esc(t('menu.game')) + '">' +
              svg(ICONS.Menu, 16) + '</button>' +
            (S.menuOpen
              ? '<div class="tf-menu">' + menuItems + '</div>'
              : '') +
          '</div>' +
        '</div>' +
        (S.notice
          ? '<div class="tf-notice" data-act="clear-notice">' + esc(S.notice) + '</div>'
          : '') +
      '</div>'
    );
  }

  // ═══════════════ 팝업 ═══════════════
  var POPUP_META = {
    chars: { titleKey: 'menu.chars', width: 'max-w-3xl' },
    res: { titleKey: 'menu.resources', width: 'max-w-lg' },
    inventions: { titleKey: 'menu.civilization', width: 'max-w-3xl' },
    log: { titleKey: 'menu.log', width: 'max-w-3xl' },
    account: { titleKey: 'menu.account', width: 'max-w-sm' },
    set: { titleKey: 'menu.settings', width: 'max-w-sm' },
  };

  // 문명 패널 — tf-civ.js로 분리(500줄 규칙·기능 분리, 2026-10-22).
  // 클로저 의존(S·t·esc…)은 생성 시점에 ctx로 주입한다.
  var civPanel = window.TF_CIV.create({ S: S, t: t, tOr: tOr, esc: esc, mt: mt,
    chipIconHtml: chipIconHtml, I: I, DATA: DATA });

  // 캐릭터 패널 — tf-chars.js로 분리(500줄·기능 분리, 2026-10-22).
  var charsPanel = window.TF_CHARS.create({ S: S, t: t, tOr: tOr, esc: esc, mt: mt,
    svg: svg, ICONS: ICONS, I: I, DATA: DATA, chipIconHtml: chipIconHtml,
    schedule: schedule,
    slots: { get hero() { return slots.hero; } } });

  // 자원·기록 패널 — tf-reslog.js로 분리(500줄 규칙, 2026-10-22).
  var resPanel = window.TF_RESLOG.create({ S: S, t: t, tOr: tOr, esc: esc, mt: mt,
    I: I, DATA: DATA, chipIconHtml: chipIconHtml, resUsable: resUsable, resStats: resStats,
    isHumanRow: isHumanRow, chatDisplayText: chatDisplayText });

  // 계정·설정 패널 — tf-set.js로 분리(500줄 규칙, 2026-10-22).
  var setPanel = window.TF_SET.create({ t: t, esc: esc, svg: svg, ICONS: ICONS,
    I: I, hasToken: hasToken });

  function popupBody(key) {
    if (key === 'chars') return charsPanel.html();
    if (key === 'res') return resPanel.html();
    if (key === 'inventions') return civPanel.html();
    if (key === 'log') return resPanel.log();
    if (key === 'account') return setPanel.account();
    return setPanel.html();
  }

  // 팝업 렌더 — 오버레이·대화상자 뼈대는 유지하고 몸통만 교체한다.
  // worldsim 틱마다 render가 도는데 루트를 통째로 다시 쓰면 입장 애니메이션
  // (tf-fade-in)이 매 틱 재시작돼 팝업이 깜빡인다.
  var popupDom = { key: null, title: null, body: null };
  function popupUpdate() {
    var slot = slots.popup;
    if (!S.popup) {
      if (popupDom.key !== null) {
        popupDom = { key: null, title: null, body: null };
        slot.innerHTML = '';
      }
      return;
    }
    var meta = POPUP_META[S.popup];
    var title = S.popup === 'account' && !hasToken() ? t('menu.login') : t(meta.titleKey);
    if (popupDom.key !== S.popup) {
      popupDom = { key: S.popup, title: title, body: popupBody(S.popup) };
      slot.innerHTML =
        '<div class="tf-overlay" data-act="close-popup">' +
          '<div class="tf-dialog ' + meta.width + '" role="dialog" aria-modal="true">' +
            '<div class="tf-dialog-head">' +
              '<h2 class="tf-dialog-title">' + esc(title) + '</h2>' +
              '<button class="tf-dialog-close" data-act="close-popup" title="' + esc(t('btn.close')) + '">' +
                svg(ICONS.X, 16) + '</button>' +
            '</div>' + popupDom.body +
          '</div>' +
        '</div>';
      return;
    }
    if (popupDom.title !== title) {
      popupDom.title = title;
      var titleEl = slot.querySelector('.tf-dialog-title');
      if (titleEl) titleEl.textContent = title;
    }
    var body = popupBody(S.popup);
    if (popupDom.body !== body) {
      popupDom.body = body;
      var dialog = slot.querySelector('.tf-dialog');
      // head 다음 몸통 루트만 교체 — 오버레이 노드는 그대로(애니메이션 유지)
      if (dialog && dialog.children.length > 1) dialog.children[1].outerHTML = body;
    }
  }

  function creditsHtml() {
    var authors = Object.keys(DATA.CREDITS.authors).map(function (name) {
      var c = DATA.CREDITS.authors[name];
      return '<div class="tf-credit-card"><div class="nm">' + esc(name) + '</div>' +
        '<div class="lc">' + esc(c.licenses.join(' · ')) + '</div></div>';
    }).join('');
    var tools = {};
    Object.keys(DATA.CREDITS.tools).forEach(function (k) { tools[k] = DATA.CREDITS.tools[k]; });
    Object.keys(DATA.CREDITS.spears).forEach(function (k) { tools[k] = DATA.CREDITS.spears[k]; });
    var toolHtml = Object.keys(tools).map(function (name) {
      var c = tools[name];
      return '<div class="tf-credit-card"><div class="nm">' + esc(name) + '</div>' +
        '<div class="lc">' + esc(c.licenses.join(' · ')) + ' — ' + esc(c.items.join(', ')) + '</div></div>';
    }).join('');
    return '<div class="tf-overlay" data-act="close-credits">' +
      '<div class="tf-dialog max-w-3xl">' +
        '<div class="tf-dialog-head">' +
          '<h2 class="tf-dialog-title">' + esc(t('set.credits')) + '</h2>' +
          '<button class="tf-dialog-close" data-act="close-credits">' + svg(ICONS.X, 16) + '</button>' +
        '</div>' +
        '<div class="tf-credits tf-scroll" data-scroll="credits">' +
          '<section class="tf-card" style="padding:1rem">' +
            '<h2>' + esc(t('credits.tiles.h')) + '</h2>' +
            '<p>' + esc(t('credits.tiles.body1')) + ' <a href="https://kenney.nl" target="_blank" rel="noreferrer">Kenney</a> ' + esc(t('credits.tiles.body2')) + '</p>' +
          '</section>' +
          '<section class="tf-card" style="padding:1rem">' +
            '<h2>' + esc(t('credits.icons.h')) + '</h2>' +
            '<p>' + esc(t('credits.icons.body1')) + ' <a href="https://game-icons.net" target="_blank" rel="noreferrer">game-icons.net</a> ' +
            esc(t('credits.icons.body2')) + ' <strong>Lorc</strong>(stone-spear, stone-axe, curvy-knife, flat-hammer, needle-drill, harpoon-trident, mantrap, spears), ' +
            '<strong>Delapouite</strong>(bow-arrow, hand-saw, sling, fishing-pole, basket, cooking-pot, plain-arrow).</p>' +
          '</section>' +
          '<section class="tf-card" style="padding:1rem">' +
            '<h2>' + esc(t('credits.sprites.h')) + '</h2>' +
            '<p style="margin-bottom:.5rem">' + esc(t('credits.sprites.body1')) +
            ' <a href="https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator" target="_blank" rel="noreferrer">Universal LPC Spritesheet Character Generator</a> ' +
            esc(t('credits.sprites.body2')) + ' ' + esc(t('credits.sprites.body3')) + '</p>' +
            '<div class="tf-credits-grid">' + authors + '</div>' +
            '<h3>' + esc(t('credits.tools.h')) + ' (docs §17.39)</h3>' +
            '<p style="margin-bottom:.5rem">' + esc(t('credits.tools.body')) + '</p>' +
            '<div class="tf-credits-grid">' + toolHtml + '</div>' +
            '<p style="margin-top:.75rem;font-size:12px;color:var(--ink-400)">' + esc(t('credits.docs')) +
            ' <a href="https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator/blob/master/README.md" target="_blank" rel="noreferrer">' +
            esc(t('credits.repo')) + '</a></p>' +
          '</section>' +
        '</div>' +
      '</div>' +
    '</div>';
  }

  // ── 외형 편집 모달(FE LookEditor 포트) ──
  function lookHtml() {
    var E = S.lookEdit;
    var tab = E.tab;
    var rows;
    if (tab === 'clothes') {
      rows = ['top', 'bottom', 'feet'].map(function (slot) {
        var label = t(slot === 'top' ? 'look.top' : slot === 'bottom' ? 'look.bottom' : 'look.feet');
        var options = E.wearOptions(slot);
        var value = E.wears[slot] || 'auto';
        return optRowHtml(label, options, value, E);
      }).join('');
    } else if (tab === 'hair') {
      rows = optRowHtml(t('look.hair'), E.opts.hairs.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.hair || E.opts.hairs[0], E) +
        optRowHtml(t('look.hairColor'), E.opts.hairColors.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.hairColor || E.opts.hairColors[0], E);
    } else {
      rows = optRowHtml(t('look.skin'), E.opts.skins.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.skin || E.opts.skins[0], E) +
        optRowHtml(t('look.eye'), E.opts.eyes.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.eye || E.opts.eyes[0], E) +
        optRowHtml(t('look.beard'), E.opts.beards.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.beard || 'none', E) +
        optRowHtml(t('look.glasses'), E.opts.glasses.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.glasses || 'none', E);
    }
    var tabs = [['clothes', 'look.tab.clothes'], ['hair', 'look.tab.hair'], ['face', 'look.tab.face']].map(function (tb) {
      return '<button class="tf-look-tab' + (tab === tb[0] ? ' on' : '') + '" data-act="look-tab" data-key="' + tb[0] + '">' + esc(t(tb[1])) + '</button>';
    }).join('');
    return '<div class="tf-overlay" data-act="close-look">' +
      '<div class="tf-look tf-scroll" data-scroll="look">' +
        '<div class="tf-look-head">' +
          '<h3 class="tf-look-title">' + esc(t('look.title')) + ' — ' + esc(E.name) + '</h3>' +
          '<button class="tf-dialog-close" data-act="close-look">' + svg(ICONS.X, 16) + '</button>' +
        '</div>' +
        '<div class="tf-look-body">' +
          frameHtml(E, 136, false) +
          '<div class="tf-look-main">' +
            '<div class="tf-look-tabs">' + tabs + '</div>' +
            '<div class="tf-look-rows">' + rows + '</div>' +
          '</div>' +
        '</div>' +
        '<div class="tf-look-actions">' +
          '<button class="tf-btn-accent" data-act="look-save" ' + (E.busy ? 'disabled' : '') + '>' + esc(t('look.save')) + '</button>' +
          '<button class="tf-btn-outline" data-act="close-look">' + esc(t('look.cancel')) + '</button>' +
        '</div>' +
        (tab === 'clothes'
          ? '<p class="tf-look-note">' + svg(ICONS.Lock, 12) + ' ' + esc(t('look.auto')) + ' · 🔒 ' + esc(t('look.locked')) + '</p>'
          : '') +
      '</div>' +
    '</div>';
  }

  /** 시트 프레임 크롭 박스(FE Frame 포트) — walk 시트 down행(2행) 0프레임. */
  function frameHtml(E, size, dim) {
    var s = size / 64;
    return '<div class="tf-frame" style="width:' + size + 'px;height:' + size + 'px">' +
      (E.frameUrl
        ? '<img src="' + E.frameUrl + '" alt="" class="' + (dim ? 'dim' : '') +
          '" style="top:' + (-64 * 2 * s) + 'px;width:' + (576 * s) + 'px;height:' + (256 * s) + 'px">'
        : '<div style="display:flex;height:100%;align-items:center;justify-content:center;font-size:10px;color:var(--ink-500)">…</div>') +
      '</div>';
  }
  function optRowHtml(label, options, value, E) {
    var idx = Math.max(0, options.findIndex(function (o) { return o.key === value; }));
    var cur = options[idx] || options[0] || { key: 'none', ko: '' };
    return '<div class="tf-optrow">' +
      '<span class="tf-optrow-label">' + esc(label) + '</span>' +
      '<button class="tf-optrow-nav" data-act="look-slide" data-dir="-1">' + svg(ICONS.ChevronLeft, 16) + '</button>' +
      '<div class="tf-optrow-main">' +
        frameHtml(E, 44, !!cur.locked) +
        '<div class="tf-optrow-name">' +
          '<div class="tf-optrow-nm' + (cur.locked ? ' locked' : '') + '">' +
            (cur.key === 'none' ? esc(t('look.none')) : esc(cur.ko)) + '</div>' +
          '<div class="tf-optrow-sub">' + (cur.locked ? '🔒 ' + esc(t('look.locked')) : (idx + 1) + '/' + options.length) + '</div>' +
        '</div>' +
      '</div>' +
      '<button class="tf-optrow-nav" data-act="look-slide" data-dir="1">' + svg(ICONS.ChevronRight, 16) + '</button>' +
    '</div>';
  }

  /** 외형 편집기 상태 — 옷(소지 기반 잠금)·헤어·얼굴 슬라이드, 저장은 PATCH look. */
  function makeLookEditor(m) {
    var opts = window.TF_LOOK.lookPartOptions(m.sex, m.age);
    var base = window.TF_LOOK.buildLook(m.id, m.sex, m.age).picks;
    var E = {
      agentId: m.id, name: m.name, sex: m.sex || 'female', age: m.age || 30,
      opts: opts,
      inv: m.inventory || {}, invClothes: m.clothingInventions || [],
      tab: 'clothes',
      parts: Object.assign({}, base, m.parts || {}),
      wears: Object.assign({}, m.wears || {}),
      frameUrl: '',
      busy: false,
      prettyPart: function (v) {
        var s = (String(v).split('_').pop() || v).replace(/([a-z])([A-Z])/g, '$1 $2');
        return s.charAt(0).toUpperCase() + s.slice(1);
      },
      setPart: function (k, v) { E.parts[k] = v; E.refreshFrame(); schedule(); },
      pickWear: function (slot, key) {
        if (key === 'auto') delete E.wears[slot];
        else E.wears[slot] = key;
        E.refreshFrame();
        schedule();
      },
      // 옷 카탈로그(FE CLOTHES 미러) — 소지 기반 잠금
      CLOTHES: [
        { item: 'grass_skirt', slot: 'bottom', cf: 0.72, look: { bottom: 'legs_skirts_plain', bottomColor: 'forest' } },
        { item: 'leather_wrap', slot: 'top', cf: 0.65, look: { top: 'torso_clothes_sleeveless2', topColor: 'leather' } },
        { item: 'woven_leather', slot: 'top', cf: 0.5, look: { top: 'torso_clothes_longsleeve2', topColor: 'walnut' } },
        { item: 'stitched_leather', slot: 'top', cf: 0.35, look: { top: 'torso_clothes_longsleeve2_cardigan', topColor: 'brown' } },
        { item: 'leather_shoes', slot: 'feet', cf: 0.9, look: { feet: 'feet_boots_basic', feetColor: 'leather' } },
      ],
      INVENTED_LOOK: function (color) { return { top: 'torso_clothes_longsleeve', topColor: color }; },
      effectiveWearPatch: function () {
        var patch = {};
        var owned = function (i) { return (E.inv[i] || 0) > 0; };
        ['top', 'bottom', 'feet'].forEach(function (slot) {
          var cands = E.CLOTHES.filter(function (c) { return c.slot === slot && owned(c.item); }).concat(
            E.invClothes.filter(function (ic) { return owned(ic.name); })
              .map(function (ic) { return { item: ic.name, cf: ic.cf, look: E.INVENTED_LOOK(ic.color) }; })
          );
          var pick = E.wears[slot];
          if (pick && owned(pick)) {
            var f = cands.filter(function (c) { return c.item === pick; })[0];
            if (f) { Object.assign(patch, f.look); return; }
          }
          var best = cands.slice().sort(function (a, b) { return a.cf - b.cf; })[0];
          if (best) Object.assign(patch, best.look);
        });
        return patch;
      },
      fullParts: function () { return Object.assign({}, E.parts, E.effectiveWearPatch()); },
      wearOptions: function (slot) {
        var owned = function (i) { return (E.inv[i] || 0) > 0; };
        return [{ key: 'auto', ko: t('look.auto') }]
          .concat(E.CLOTHES.filter(function (c) { return c.slot === slot; }).map(function (c) {
            return { key: c.item, ko: tOr('item.' + c.item, c.item), locked: !owned(c.item) };
          }))
          .concat(E.invClothes.map(function (ic) { return { key: ic.name, ko: ic.name }; }));
      },
      refreshFrame: function () {
        window.TF_LOOK.charSheetUrl({ uuid: E.agentId, sex: E.sex, age: E.age, parts: E.fullParts() })
          .then(function (u) { if (S.lookEdit === E) { E.frameUrl = u; schedule(); } })
          .catch(function () {});
      },
    };
    E.refreshFrame();
    return E;
  }

  // ═══════════════ 렌더 본체 ═══════════════
  function collectScrolls(root) {
    var out = {};
    root.querySelectorAll('[data-scroll]').forEach(function (el) {
      out[el.getAttribute('data-scroll')] = el.scrollTop;
    });
    return out;
  }
  function applyScrolls(root, saved) {
    root.querySelectorAll('[data-scroll]').forEach(function (el) {
      var k = el.getAttribute('data-scroll');
      if (saved[k] != null) el.scrollTop = saved[k];
    });
  }

  // 섹션 슬롯 — worldsim 틱마다 render가 도므로 루트를 통째로 다시 쓰면
  // 팝업(tf-fade-in)·히어로 카드(tf-hero-in·걷기)의 입장 애니메이션이
  // 매 틱 재시작돼 깜빡인다. 슬롯을 고정하고 바뀐 섹션만 교체하며, 팝업·
  // 히어로는 뼈대 유지 + 값 갱신(heroUpdate·popupUpdate)으로 렌더한다.
  var SLOT_ORDER = ['topbar', 'hero', 'popup', 'credits', 'look'];
  var slots = {};
  var slotHtml = {};   // name → 마지막으로 쓴 HTML — 문자열 비교로 교체 차단
  function ensureSlots(root) {
    SLOT_ORDER.forEach(function (name) {
      var el = slots[name];
      if (!el || !root.contains(el)) {
        el = root.querySelector(':scope > [data-slot="' + name + '"]');
        if (!el) {
          el = document.createElement('div');
          el.setAttribute('data-slot', name);
          root.appendChild(el);
        }
        slots[name] = el;
        slotHtml[name] = null;
      }
    });
  }
  function setSlot(name, html) {
    if (slotHtml[name] === html) return;
    slotHtml[name] = html;
    slots[name].innerHTML = html;
  }

  function render() {
    var root = ensureRoot();
    ensureSlots(root);
    var saved = collectScrolls(root);
    var focus = document.activeElement && document.activeElement.id;
    setSlot('topbar', topbarHtml());
    charsPanel.heroUpdate();
    popupUpdate();
    setSlot('credits', S.creditsOpen ? creditsHtml() : '');
    setSlot('look', S.lookEdit ? lookHtml() : '');
    applyScrolls(root, saved);
    if (focus) {
      var el = document.getElementById(focus);
      if (el && document.activeElement !== el) {
        el.focus();
        if (el.setSelectionRange && el.value != null) {
          try { el.setSelectionRange(el.value.length, el.value.length); } catch (e) { /* pass */ }
        }
      }
    }
    wireJoinInputs();
  }

  /** 참여 폼 입력 — 타이핑 중 재렌더 금지(포커스 유지), 상태만 갱신. */
  function wireJoinInputs() {
    var name = document.getElementById('tf-join-name');
    var age = document.getElementById('tf-join-age');
    var sex = document.getElementById('tf-join-sex');
    var persona = document.getElementById('tf-join-persona');
    if (name) name.oninput = function () { S.joinForm.name = name.value; };
    if (age) age.oninput = function () {
      S.joinForm.age = age.value.replace(/\D/g, '').replace(/^0+/, '').slice(0, 2);
      if (age.value !== S.joinForm.age) age.value = S.joinForm.age;
    };
    if (sex) sex.onchange = function () { S.joinForm.sex = sex.value; schedule(); };
    if (persona) persona.oninput = function () { S.joinForm.persona = persona.value; };
  }

  // ═══════════════ 이벤트 위임 ═══════════════
  document.addEventListener('click', function (ev) {
    var el = ev.target.closest ? ev.target.closest('[data-act]') : null;
    if (S.menuOpen && !el && !ev.target.closest('.tf-menu-anchor')) {
      S.menuOpen = false;
      schedule();
      return;
    }
    if (!el) return;
    var act = el.getAttribute('data-act');
    var key = el.getAttribute('data-key');
    if (act === 'menu') { S.menuOpen = !S.menuOpen; schedule(); }
    else if (act === 'popup') { openPopup(key); }
    else if (act === 'clear-notice') { S.notice = ''; schedule(); }
    else if (act === 'hero-close') {
      // 선택 해제 — 단일 소스(Rust Selection)에 빈 요청 → 링·추적·채팅 독 함께 정리
      if (window.terrafolk && window.terrafolk.selectAgent) window.terrafolk.selectAgent('');
    }
    else if (act === 'hero-cam') {
      // 재추적 — 같은 id 재선택도 epoch로 구분돼 카메라가 다시 따라온다(pick.rs)
      if (S.selected && window.terrafolk && window.terrafolk.selectAgent) {
        window.terrafolk.selectAgent(S.selected);
      }
    }
    else if (act === 'hero-chat') {
      // 대화 — ui.js 채팅 로그 펼치고 입력에 포커스
      var head = document.querySelector('.tf-chathead');
      if (head && !head.classList.contains('on')) head.click();
      var inp = document.querySelector('.tf-chatdock .tf-input');
      if (inp) inp.focus();
    }
    else if (act === 'hero-detail') { openPopup('chars'); }
    else if (act === 'close-popup') {
      if (ev.target === el) { S.popup = null; schedule(); }   // 백드롭·X만 — 내부 클릭 전파 무시
    }
    else if (act === 'select') {
      S.selected = el.getAttribute('data-id');
      S.selRelic = null;
      // 선택 단일 소스 — Rust Selection으로 요청(링·카메라 추적·채팅 바 ui.js)
      if (window.terrafolk && window.terrafolk.selectAgent) window.terrafolk.selectAgent(S.selected);
      onSelected();
      schedule();
    }
    else if (act === 'preset') {
      var p = I.PERSONA_PRESETS.filter(function (x) { return x.key === key; })[0];
      if (p) { S.joinForm.persona = p.text; schedule(); }
    }
    else if (act === 'join') { join(); }
    else if (act === 'look-edit') {
      var m = S.mineDetail.filter(function (x) { return x.id === el.getAttribute('data-id'); })[0];
      if (m) { S.lookEdit = makeLookEditor(m); schedule(); }
    }
    else if (act === 'look-tab') { if (S.lookEdit) { S.lookEdit.tab = key; schedule(); } }
    else if (act === 'look-save') { lookSave(); }
    else if (act === 'close-look') { if (ev.target === el) { S.lookEdit = null; schedule(); } }
    else if (act === 'close-relic') { S.selRelic = null; schedule(); }
    else if (act === 'sel-inv') { S.selInv = el.getAttribute('data-name'); schedule(); }
    else if (act === 'close-inv') { if (ev.target === el) { S.selInv = null; schedule(); } }
    else if (act === 'logcat') { S.logCat = key; schedule(); }
    else if (act === 'locale') { I.setLocale(key); schedule(); }
    else if (act === 'credits') { S.creditsOpen = true; schedule(); }
    else if (act === 'close-credits') { if (ev.target === el) { S.creditsOpen = false; schedule(); } }
    else if (act === 'logout') { logout(); }
  });

  // look-slide는 행(row) 단위로 처리 — 캡처 단계에서 클릭된 optrow의 순서를 읽는다.
  // 잠긴 옵션(미보유 의상)은 볼 수만 있고 선택은 스킵(FE와 동일).
  document.addEventListener('click', function (ev) {
    var nav = ev.target.closest ? ev.target.closest('[data-act="look-slide"]') : null;
    if (!nav || !S.lookEdit) return;
    var row = nav.closest('.tf-optrow');
    if (!row) return;
    var idx = Array.prototype.indexOf.call(row.parentNode.children, row);
    var E = S.lookEdit;
    var tab = E.tab;
    var spec;
    if (tab === 'clothes') spec = { slot: ['top', 'bottom', 'feet'][idx] };
    else if (tab === 'hair') spec = { part: ['hair', 'hairColor'][idx] };
    else spec = { part: ['skin', 'eye', 'beard', 'glasses'][idx] };
    var dir = Number(nav.getAttribute('data-dir'));
    var options, value, apply;
    if (spec.slot) {
      options = E.wearOptions(spec.slot);
      value = E.wears[spec.slot] || 'auto';
      apply = function (k) { E.pickWear(spec.slot, k); };
    } else {
      var list = E.opts[spec.part] || [];
      options = list.map(function (v) { return { key: v, ko: E.prettyPart(v) }; });
      value = E.parts[spec.part] || (options[0] && options[0].key) || 'none';
      apply = function (k) { E.setPart(spec.part, k); };
    }
    var i = Math.max(0, options.findIndex(function (o) { return o.key === value; }));
    var next = options[(i + dir + options.length) % options.length];
    if (next && !next.locked) apply(next.key);
    else schedule();
  }, true);

  var modalEscLatch = false;
  document.addEventListener('keydown', function (ev) {
    if (ev.key === 'Escape') {
      if (S.lookEdit) { S.lookEdit = null; modalEscLatch = true; schedule(); }
      else if (S.creditsOpen) { S.creditsOpen = false; modalEscLatch = true; schedule(); }
      else if (S.popup) { S.popup = null; modalEscLatch = true; schedule(); }
      else if (S.menuOpen) { S.menuOpen = false; modalEscLatch = true; schedule(); }
    }
  });

  // ═══════════════ REST ═══════════════
  function notify(msg) {
    S.notice = String(msg || '');
    schedule();
    if (I.getLocale() !== 'en' && S.notice) {
      I.translateText(I.getLocale(), S.notice).then(function (tr) {
        if (tr !== S.notice) { S.notice = tr; schedule(); }
      }).catch(function () {});
    }
  }

  function openPopup(key) {
    S.menuOpen = false;
    S.popup = key;
    schedule();
    if (key === 'chars') { loadRelations(); onSelected(); }
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

  function lookSave() {
    var E = S.lookEdit;
    if (!E || E.busy) return;
    E.busy = true;
    schedule();
    // 옷 슬롯은 wears가 관장 — 수동 parts의 옷 잔존값은 정리해 파생이 우선하게.
    // wears는 에디터 선택(E.wears, 미지정 슬롯=auto 초기화)을 그대로 — 하드코딩
    // auto면 사용자 선택·기존 저장값이 전부 지워진다(FE LookEditor 규약)
    var partsOut = Object.assign({}, E.parts);
    ['top', 'topColor', 'bottom', 'bottomColor', 'feet', 'feetColor'].forEach(function (k) { delete partsOut[k]; });
    var wearsOut = { top: E.wears.top || 'auto', bottom: E.wears.bottom || 'auto', feet: E.wears.feet || 'auto' };
    authFetch('/api/worldsim/agents/' + encodeURIComponent(E.agentId) + '/look', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ parts: partsOut, wears: wearsOut }),
    }).then(function (r) {
      if (!r.ok) return r.json().then(function (j) { throw new Error(j.message || j.detail || 'save failed'); });
      return r.json();
    }).then(function () {
      S.lookEdit = null;
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

  // ═══════════════ feed — worldsim.js가 호출 ═══════════════
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

  function feedFrame(f) {
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
        x: a.x != null ? a.x : (cur ? cur.x : 0),
        y: a.y != null ? a.y : (cur ? cur.y : 0),
        mine: cur ? cur.mine : false,
      };
    });
    Object.keys(S.live).forEach(function (k) { if (!seen[k]) delete S.live[k]; });
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
    var sig = liveSignature();
    if (sig !== S.liveSig) {
      S.liveSig = sig;
      loadMine();
    }
    // 선택 해제는 Rust 단일 소스(tf.onPickAgent(null))에 맡긴다 — 프레임 공백
    // (재접속·드랍)으로 S.live가 잠깐 비어도 카드·추적이 튕기지 않는다.
    // Rust는 AgentGrace 유예 후 실제 철거 시에만 null을 통보한다.
    schedule();
  }
  function liveSignature() {
    return Object.keys(S.live).sort().join(',');
  }
  function feedEvent(e) {
    S.events.push(Object.assign({ id: nextRowId(), ts: new Date().toISOString() }, e));
    if (S.events.length > 400) S.events.splice(0, S.events.length - 400);
    // 스폰 이름 시드 — 접속 후 합류한 주민도 이름으로
    if (e.kind === 'spawn' && e.agent_id && e.payload && e.payload.name) {
      var cur = S.live[e.agent_id];
      if (cur && cur.name !== String(e.payload.name)) cur.name = String(e.payload.name);
    }
    if (S.popup === 'log' || S.popup === 'inventions') schedule();
  }
  function feedChat(c) {
    S.chat.push(Object.assign({ id: nextRowId(), orig: c.text, ts: new Date().toISOString() }, c));
    if (S.chat.length > 120) S.chat.splice(0, S.chat.length - 120);
    if (S.popup === 'log') schedule();
  }
  function feedScope(scope) {
    S.scope = scope;
    if (scope && scope.kind === 'user') loadMine();
    schedule();
  }

  // ── Rust 픽 브리지 — 빈 땅 클릭 좌표로 유물 카드(§17.38).
  //    주민 픽킹은 Rust pick.rs가直接 하고 tf.onPickAgent로 돌아온다.
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

  // ── Rust 선택 통보 수신 — ui.js가 먼저 등록한 핸들러를 chaining으로 이어받는다.
  //    (로드 순서: ui.js → tf-ui.js — 채팅 바와 팝업이 같은 선택 신호를 함께 받는다)
  (function chainPickAgent() {
    var tf = window.terrafolk;
    if (!tf) return;
    var prev = tf.onPickAgent;
    tf.onPickAgent = function (id) {
      if (prev) { try { prev(id); } catch (e) { console.error('[tf-ui] onPickAgent 체인 오류', e); } }
      S.selected = id || null;
      S.selRelic = null;
      onSelected();
      schedule();   // 히어로 카드 즉시 반영 — 단선 시엔 틱이 안 오니 여기서 갱신
    };
  })();

  // ═══════════════ 공개 API ═══════════════
  // modalEsc — 이번 Escape로 DOM 모달을 닫았는지 읽고 지운다(Rust pick.rs).
  // 같은 키 입력에서 JS가 먼저 모달을 닫으므로 프레임 시점의 uiModal 읽기로는
  // 판별 불가 — 래치로 1회 소진 계약.
  var tfb = window.terrafolk;
  if (tfb) tfb.modalEsc = function () { var v = modalEscLatch; modalEscLatch = false; return v; };

  // 라이브 프라임 로딩 캡션(index.html 정적 폴백 텍스트)을 사전 언어로 교체 —
  // 키(boot.loading)는 ../terrafolk 정본 사전 등록 후 갱신된다(tOr 폴백 유지).
  document.addEventListener('DOMContentLoaded', function () {
    var cap = document.querySelector('.tf-boot-caption');
    if (cap) cap.textContent = tOr('boot.loading', '세계에 접속하는 중…');
  });
  window.TF_UI = {
    state: S,
    feed: {
      frame: feedFrame,
      event: feedEvent,
      chat: feedChat,
      scope: feedScope,
    },
    open: openPopup,
    openRelic: function (r) { S.selRelic = r; openPopup('res'); },
    pickAt: pickAt,
    seedBacklogs: loadBacklogs,
    notify: notify,
    setNet: function (live) { S.netLive = live; },
    render: schedule,
    // 소지 칩 아이콘 HTML — ui.js 우측 레일 소지품 패널이 재사용(동일 룩 유지)
    chipIcon: chipIconHtml,
  };
  I.onLocale(function () { schedule(); });

  // 루트 뼈대 — worldsim.js 접속 전에도 상단 바가 뜬다(로케일만으로 구성)
  schedule();
})();
