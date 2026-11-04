// Terrafolk ☰ 메뉴·중앙 팝업 DOM 오버레이 코어 — FE worldsim 화면(상단 바·
// 메뉴·팝업 6종·크레딧)의 바닐라 JS 포트. 상태 S와 렌더 슬롯·이벤트 위임을
// 소유하고, 협력 모듈에 ctx를 주입해 조립한다(index.html 로드 순서 계약 —
// 전부 tf-ui.js보다 앞):
//   tf-civ·tf-chars·tf-reslog·tf-set  패널 렌더(2026-10-22 분리)
//   tf-rest                            REST 전부·공지 바·내 주민 id(2026-11-03)
//   tf-feed                            worldsim feed.* 수신·침전·유물 카드(2026-11-03)
//   tf-lookedit                        외형 편집기 UI·상호작용(2026-11-03)
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

  // ═══════════════ 상단 바 ═══════════════
  function ensureRoot() {
    var root = document.getElementById('tf-ui');
    if (!root) {
      root = document.createElement('div');
      root.id = 'tf-ui';
      document.body.appendChild(root);
      // 채팅 도크(ui.js가 먼저 로드돼 body에 붙은 경우)를 팔레트 스코프로
      // 이양 — #tf-ui 밖에선 --ink-* var이 미해결이라 입력 글자가 검정 실종.
      var dock = document.querySelector('.tf-chatdock');
      if (dock && dock.parentElement === document.body) root.appendChild(dock);
    }
    return root;
  }

  // ── 상단바 — 정보(시계·해시계·날씨·인구)는 bevy_ui 공용 HUD로 이관
  // (2026-10-25 통일 2호). DOM은 ☰ 메뉴·모드 칩(관전/참가 — DOM 인증 전용
  // 정보)·공지만 담는다 — 배경은 bevy HUD가 제공하므로 투명 라이트.
  // 우측 클러스터(2026-11-04 상용화): 모드 칩=상태 점 필, ☰=글래스 스쿼클,
  // 드롭다운=아이콘 칩 + 본문/시스템 구분선 + 등장 애니메이션.
  function topbarHtml() {
    var isUser = S.scope && S.scope.kind === 'user';
    var login = !hasToken();
    var item = function (m) {
      var isLogin = m[0] === 'account' && login;
      return '<button class="tf-menu-item' + (isLogin ? ' login' : '') + '" role="menuitem"' +
        ' data-act="popup" data-key="' + m[0] + '">' +
        '<span class="tf-menu-ico">' + svg(m[2], 15) + '</span>' +
        '<span class="tf-menu-txt">' + esc(m[1]) + '</span></button>';
    };
    var menuItems =
      [['chars', t('menu.chars'), ICONS.Users],
       ['res', t('menu.resources'), ICONS.TreePine],
       ['inventions', t('menu.civilization'), ICONS.Landmark],
       ['log', t('menu.log'), ICONS.ScrollText]].map(item).join('') +
      '<div class="tf-menu-sep" role="separator"></div>' +
      [['set', t('menu.settings'), ICONS.Settings],
       ['account', login ? t('menu.login') : t('menu.account'), login ? ICONS.LogIn : ICONS.User]]
        .map(item).join('');
    return (
      '<div class="tf-topbar tf-topbar-lite">' +
        '<span class="tf-chip tf-chip-mode' + (isUser ? ' tf-chip-user' : '') + '">' +
          '<i class="dot' + (S.netLive ? '' : ' off') + '"></i>' +
          esc(isUser ? t('world.joined') : t('world.spectate')) + '</span>' +
        '<div class="tf-menu-anchor">' +
          '<button class="tf-menu-btn' + (S.menuOpen ? ' on' : '') + '" data-act="menu"' +
            ' aria-haspopup="menu" aria-expanded="' + (S.menuOpen ? 'true' : 'false') + '"' +
            ' title="' + esc(t('menu.game')) + '">' +
            svg(ICONS.Menu, 17) + '</button>' +
          (S.menuOpen
            ? '<div class="tf-menu" role="menu">' + menuItems + '</div>'
            : '') +
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

  // ── 협력 모듈 조립(로드 순서 계약: 전부 이 시점에 정의됨) ──
  var rest = window.TF_REST.create({ S: S, t: t, I: I, schedule: schedule,
    liveSignature: function () { return Object.keys(S.live).sort().join(','); } });
  var hasToken = rest.hasToken;

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
    I: I, DATA: DATA, chipIconHtml: chipIconHtml });

  // 크레딧 화면 — tf-credits.js로 분리(500줄 규칙, 2026-11-03).
  var creditsMod = window.TF_CREDITS.create({ t: t, esc: esc, svg: svg, ICONS: ICONS, DATA: DATA });

  // 계정·설정 패널 — tf-set.js로 분리(500줄 규칙, 2026-10-22).
  var setPanel = window.TF_SET.create({ t: t, esc: esc, svg: svg, ICONS: ICONS,
    I: I, hasToken: hasToken });

  // 외형 편집기 — tf-lookedit.js(2026-11-03). 저장·미리보기는 tf-rest 계층.
  var lookedit = window.TF_LOOKEDIT.create({ S: S, t: t, tOr: tOr, esc: esc, svg: svg,
    ICONS: ICONS, schedule: schedule,
    save: rest.lookSave, previewPush: rest.localLookPush });

  // 라이브 프레임 수신 — tf-feed.js(2026-11-03). /me·내 집 해제는 tf-rest 계층.
  var feedmod = window.TF_FEED.create({ S: S, I: I, schedule: schedule,
    nextRowId: rest.nextRowId, openPopup: openPopup,
    loadMine: rest.loadMine, clearMine: rest.clearMine });

  function openPopup(key) {
    S.menuOpen = false;
    S.popup = key;
    schedule();
    if (key === 'chars') { rest.loadRelations(); rest.onSelected(); }
  }

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

  /** 편집기 닫기 — 남은 디바운스 즉시 실행(마지막 선택 유실 방지) 후 포커스 해제. */
  function closeLookEditor() {
    var E = S.lookEdit;
    if (!E) return;
    if (E.saveTimer) { clearTimeout(E.saveTimer); E.saveTimer = 0; rest.lookSave(E.tab === 'clothes'); }
    rest.lookFocus(false);
    S.lookEdit = null;
    schedule();
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
    setSlot('credits', S.creditsOpen ? creditsMod.html() : '');
    setSlot('look', S.lookEdit ? lookedit.html() : '');
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
    charsPanel.wireJoinInputs();
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
      rest.onSelected();
      schedule();
    }
    else if (act === 'preset') {
      var p = I.PERSONA_PRESETS.filter(function (x) { return x.key === key; })[0];
      if (p) { S.joinForm.persona = p.text; schedule(); }
    }
    else if (act === 'join') { rest.join(); }
    else if (act === 'look-edit') {
      var m = S.mineDetail.filter(function (x) { return x.id === el.getAttribute('data-id'); })[0];
      if (m) {
        S.lookEdit = lookedit.makeEditor(m);
        // 편집은 풀스크린 전용 화면으로 — 주민 목록 팝업이 뒤에 겹쳐
        // 난잡해 보이는 문제(2026-10-23 모바일 보고). 카메라 팔로우로
        // 캐릭터를 화면 상단에 크게 띄운다(밤은 러스트가 정오 조명으로).
        S.popup = null;
        rest.lookFocus(true, m.id);
        schedule();
      }
    }
    else if (act === 'look-tab') { if (S.lookEdit) { S.lookEdit.tab = key; schedule(); } }
    else if (act === 'look-save') { rest.lookSave(); }
    else if (act === 'look-pick') {
      // 팔레트 스와치 즉시 선택 — 미리보기·자동 저장은 슬라이드와 동일 경로
      var E0 = S.lookEdit;
      if (E0) {
        E0.setPart(el.getAttribute('data-part'), key);
        rest.localLookPush(E0);
        clearTimeout(E0.saveTimer);
        E0.saveTimer = setTimeout(function () { rest.lookSave(false); }, 500);
      }
    }
    else if (act === 'close-look') { if (ev.target === el) { closeLookEditor(); } }
    else if (act === 'close-relic') { S.selRelic = null; schedule(); }
    else if (act === 'sel-inv') { S.selInv = el.getAttribute('data-name'); schedule(); }
    else if (act === 'close-inv') { if (ev.target === el) { S.selInv = null; schedule(); } }
    else if (act === 'logcat') { S.logCat = key; schedule(); }
    else if (act === 'locale') { I.setLocale(key); schedule(); }
    else if (act === 'credits') { S.creditsOpen = true; schedule(); }
    else if (act === 'close-credits') { if (ev.target === el) { S.creditsOpen = false; schedule(); } }
    else if (act === 'logout') { rest.logout(); }
  });

  var modalEscLatch = false;
  document.addEventListener('keydown', function (ev) {
    if (ev.key === 'Escape') {
      if (S.lookEdit) { closeLookEditor(); modalEscLatch = true; }
      else if (S.creditsOpen) { S.creditsOpen = false; modalEscLatch = true; schedule(); }
      else if (S.popup) { S.popup = null; modalEscLatch = true; schedule(); }
      else if (S.menuOpen) { S.menuOpen = false; modalEscLatch = true; schedule(); }
    }
  });

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
      rest.onSelected();
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
  // 키(boot.loading)는 ../terrafolk-2d-legacy 정본 사전 등록 후 갱신된다(tOr 폴백 유지).
  document.addEventListener('DOMContentLoaded', function () {
    var cap = document.querySelector('.tf-boot-caption');
    if (cap) cap.textContent = tOr('boot.loading', '세계에 접속하는 중…');
  });
  window.TF_UI = {
    state: S,
    feed: {
      frame: feedmod.frame,
      event: feedmod.event,
      chat: feedmod.chat,
      scope: feedmod.scope,
    },
    open: openPopup,
    openRelic: function (r) { S.selRelic = r; openPopup('res'); },
    pickAt: feedmod.pickAt,
    seedBacklogs: rest.loadBacklogs,
    notify: rest.notify,
    pushMine: rest.pushMine,
    setNet: function (live) { S.netLive = live; },
    render: schedule,
    // 소지 칩 아이콘 HTML — ui.js 우측 레일 소지품 패널이 재사용(동일 룩 유지)
    chipIcon: chipIconHtml,
  };
  I.onLocale(function () { schedule(); });

  // 루트 뼈대 — worldsim.js 접속 전에도 상단 바가 뜬다(로케일만으로 구성)
  schedule();
})();
