// Terrafolk 캐릭터 패널 — tf-ui.js에서 기능 분리(파일당 500줄 규칙, 2026-10-22).
// 캐릭터 목록·참여(조인 폼·페르소나)·내 캐릭터 외형 편집·히어로(선택 상세)
// 렌더를 소유. 상호작용(data-act)은 tf-ui 디스패처에 남는다(HTML만 이동).
// 클로저 의존은 TF_CHARS.create(ctx)로 tf-ui.js가 주입 — tf-ui.js보다 먼저 로드.
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, t = ctx.t, tOr = ctx.tOr, esc = ctx.esc, mt = ctx.mt,
        svg = ctx.svg, ICONS = ctx.ICONS, I = ctx.I, DATA = ctx.DATA,
        chipIconHtml = ctx.chipIconHtml, schedule = ctx.schedule,
        slots = ctx.slots;
  function charsHtml() {
    var isUser = S.scope && S.scope.kind === 'user';
    var agents = Object.keys(S.live).map(function (k) { return S.live[k]; });
    var left;

    if (isUser) {
      var presets = I.PERSONA_PRESETS.map(function (p) {
        return '<button class="tf-preset' + (S.joinForm.persona === p.text ? ' on' : '') +
          '" data-act="preset" data-key="' + p.key + '" title="' + esc(p.text) + '">' +
          p.emoji + ' ' + esc(t('preset.' + p.key)) + '</button>';
      }).join('');
      var mine = S.mineDetail.length
        ? '<div style="margin-top:.375rem;display:flex;flex-direction:column;gap:.25rem">' +
            S.mineDetail.map(function (m) {
              return '<button class="tf-btn-ghost" data-act="look-edit" data-id="' + esc(m.id) + '">' +
                '<span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + esc(m.name) + '</span>' +
                '<span class="edit">' + esc(t('look.edit')) + '</span></button>';
            }).join('') + '</div>'
        : '';
      left =
        '<section class="tf-card">' +
          '<h3>' + svg(ICONS.UserPlus, 14) + ' ' + esc(t('panel.create.title')) + '</h3>' +
          '<div style="margin-top:.5rem;display:flex;flex-direction:column;gap:.375rem">' +
            '<input class="tf-input" id="tf-join-name" placeholder="' + esc(t('panel.create.name')) +
              '" value="' + esc(S.joinForm.name) + '">' +
            '<div class="tf-join-row">' +
              '<input class="tf-input age" id="tf-join-age" inputmode="numeric" placeholder="' + esc(t('panel.create.age')) +
                '" title="10~80" value="' + esc(S.joinForm.age) + '">' +
              '<select class="tf-input sm" id="tf-join-sex">' +
                '<option value="female"' + (S.joinForm.sex === 'female' ? ' selected' : '') + '>' + esc(t('panel.create.female')) + '</option>' +
                '<option value="male"' + (S.joinForm.sex === 'male' ? ' selected' : '') + '>' + esc(t('panel.create.male')) + '</option>' +
              '</select>' +
            '</div>' +
            '<input class="tf-input" id="tf-join-persona" placeholder="' + esc(t('panel.create.persona')) +
              '" value="' + esc(S.joinForm.persona) + '">' +
            '<div class="tf-presets">' + presets + '</div>' +
            '<button class="tf-btn-accent" data-act="join" ' +
              (S.busy || !S.joinForm.name.trim() ? 'disabled' : '') + '>' + esc(t('panel.create.submit')) + '</button>' +
            mine +
          '</div>' +
        '</section>';
    } else {
      left =
        '<section class="tf-card" style="font-size:12px;color:var(--ink-400)">' +
          '<h3>' + svg(ICONS.LogIn, 14) + ' ' + esc(t('panel.spectate.title')) + '</h3>' +
          '<p style="margin:.375rem 0 0;line-height:1.625">' + esc(t('panel.spectate.desc')) + '</p>' +
        '</section>';
    }

    var residents = agents.map(function (a) {
      var act = a.actType || a.status;
      return '<li><button class="tf-resident' + (S.selected === a.id ? ' on' : '') +
        '" data-act="select" data-id="' + esc(a.id) + '">' +
        '<div class="tf-resident-row">' +
          '<span class="tf-resident-name">' + esc(a.name) +
            (a.mine ? '<span class="tf-mine-badge">' + esc(t('mine.badge')) + '</span>' : '') + '</span>' +
          '<span class="tf-resident-act">' + (act ? esc(tOr('act.' + act, act)) : '') + '</span>' +
        '</div>' +
        (a.goalText ? '<div class="tf-resident-goal">' + esc(mt(a.goalText)) + '</div>' : '') +
        '</button></li>';
    }).join('');
    left +=
      '<section class="tf-card">' +
        '<h3>' + esc(t('panel.residents', { n: agents.length })) + '</h3>' +
        '<ul class="tf-residents">' +
          (agents.length ? residents : '<li style="padding:.25rem .5rem;font-size:12px;color:var(--ink-400)">' + esc(t('panel.residents.empty')) + '</li>') +
        '</ul>' +
      '</section>';

    var right = selDetailHtml();
    return '<div class="tf-chars tf-scroll" data-scroll="chars">' +
      '<div class="tf-chars-left">' + left + '</div>' +
      '<div class="tf-chars-right">' + right + '</div>' +
    '</div>';
  }

  function selDetailHtml() {
    var sel = S.selected ? S.live[S.selected] : null;
    if (!sel) {
      return '<div class="tf-select-hint">' + esc(t('panel.select.hint')) + '</div>';
    }
    var needs = HERO_NEEDS.map(function (pair) {
        var v = Number((sel.needs || {})[pair[1]] != null ? sel.needs[pair[1]] : 0);
        var pct = Math.min(100, v);
        return '<div class="tf-need">' +
          '<span class="tf-need-label">' + esc(t(pair[0])) + '</span>' +
          '<div class="tf-bar"><div class="tf-bar-fill' + (v > 70 ? ' hot' : '') +
            '" style="width:' + pct + '%"></div></div>' +
          '<span class="tf-need-val">' + Math.round(v) + '</span>' +
        '</div>';
      }).join('');

    var invEntries = Object.keys(sel.inv || {}).filter(function (k) { return (sel.inv[k] || 0) > 0; });
    var inv = invEntries.length
      ? invEntries.map(function (item) {
          return '<span class="tf-chipbox" title="' + esc(tOr('item.' + item, item)) + '">' +
            chipIconHtml(item) + '<span>' + esc(tOr('item.' + item, item)) + '</span>' +
            '<span class="tf-qty">×' + sel.inv[item] + '</span></span>';
        }).join('')
      : '<span class="tf-empty">' + esc(t('panel.inventory.empty')) + '</span>';

    var hist = S.history.length
      ? S.history.map(function (h) {
          return '<div class="tf-hist">' +
            '<span class="tf-hist-time">' + esc(I.fmtTime(h.ts)) + '</span>' +
            '<span class="tf-hist-type" style="color:' + (I.HIST_COLOR[h.type] || 'var(--ink-300)') + '">' +
              esc(tOr('hist.' + h.type, h.type)) + '</span>' +
            '<span class="tf-hist-text">' + esc(mt(h.orig || h.text)) + '</span>' +
          '</div>';
        }).join('')
      : '<div class="tf-empty">' + esc(t('panel.history.empty')) + '</div>';

    var rels = S.relations
      .filter(function (r) { return r.a_id === sel.id || r.b_id === sel.id; })
      .sort(function (a, b) { return b.warmth - a.warmth; })
      .slice(0, 6);
    var relHtml = rels.length
      ? rels.map(function (r) {
          var otherId = r.a_id === sel.id ? r.b_id : r.a_id;
          var nm = (S.live[otherId] && S.live[otherId].name) || otherId;
          return '<div class="tf-rel" title="' + esc(t('panel.relations.chats', { n: r.chats, m: r.trades })) + '">' +
            '<span class="tf-rel-name" style="color:' + I.agentColor(otherId) + '">' + esc(nm) + '</span>' +
            '<div class="tf-bar"><div class="tf-bar-fill" style="width:' + r.warmth + '%;background:hsl(' +
              Math.round(r.warmth * 1.2) + ',65%,55%)"></div></div>' +
            '<span class="tf-rel-val">' + esc(t('rel.' + I.relBucket(r.warmth))) + ' ' + r.warmth + '</span>' +
          '</div>';
        }).join('')
      : '<div class="tf-empty">' + esc(t('panel.relations.empty')) + '</div>';

    var memHtml = '';
    if (S.scope && S.scope.kind === 'user') {
      var rows = S.memories.length
        ? S.memories.map(function (m) {
            return '<div class="tf-mem">' +
              '<span class="tf-mem-kind ' + esc(m.kind) + '">' + esc(tOr('mem.' + m.kind, m.kind)) + '</span>' +
              '<span style="color:var(--ink-300)">' + esc(mt(m.orig || m.text)) + '</span>' +
            '</div>';
          }).join('')
        : '<div class="tf-empty">' + esc(t('panel.memories.empty')) + '</div>';
      memHtml =
        '<section class="tf-card">' +
          '<h3>' + svg(ICONS.MapPin, 14) + ' ' + esc(t('panel.memories')) + '</h3>' +
          '<div class="tf-scroll" data-scroll="mem" style="margin-top:.375rem;max-height:10rem;overflow-y:auto;display:flex;flex-direction:column;gap:.25rem">' +
            rows + '</div>' +
        '</section>';
    }

    return (
      '<section class="tf-card">' +
        '<h3>' + esc(sel.name) + ' — ' + esc(t('panel.status')) + '</h3>' +
        '<div style="margin-top:.375rem;display:flex;flex-direction:column;gap:.375rem">' + needs + '</div>' +
      '</section>' +
      '<section class="tf-card">' +
        '<h3>' + esc(t('panel.inventory')) + '</h3>' +
        '<div class="tf-invs">' + inv + '</div>' +
      '</section>' +
      '<section class="tf-card">' +
        '<h3>' + esc(t('panel.history')) + '</h3>' +
        '<div class="tf-scroll" data-scroll="hist" style="margin-top:.375rem;max-height:9rem;overflow-y:auto;display:flex;flex-direction:column;gap:.25rem">' + hist + '</div>' +
      '</section>' +
      '<section class="tf-card">' +
        '<h3>' + esc(t('panel.relations')) + '</h3>' +
        '<div style="margin-top:.375rem;display:flex;flex-direction:column;gap:.375rem">' + relHtml + '</div>' +
      '</section>' + memHtml
    );
  }

  // ═══════════════ 주민 히어로 카드(맵 클릭 선택 시 좌하단 컨텍스트 패널) ═══════════════
  // 상용 시뮬 관전 패턴 — 선택 즉시 신원·행동·욕구·소지를 한눈에, 주요 동작
  // (카메라 재추적·대화·상세)은 원클릭. 초상은 LPC walk 시트(64px 프레임)를
  // CSS steps로 애니메이션 — 데이터는 TF_LOOK 캐시가 재사용된다.
  var heroPortraits = {};   // id → dataURL(charSheetUrl 완료 시점 갱신)

  /** 초상 성별 — villager.rs model_path_for와 동일 판정. lookJson.sex는 첫
   *  글자 F/f=여성, 없으면 id 바이트 해시 홀수=여성(31은 홀수라 u64 래핑
   *  곱의 패리티는 바이트 패리티 합과 같다) — 3D 모델과 초상이 어긋나지
   *  않는다. */
  function heroSexOf(id, look) {
    if (look && look.sex && String(look.sex).charAt(0)) {
      return /^[Ff]/.test(String(look.sex)) ? 'female' : 'male';
    }
    var p = 0;
    for (var i = 0; i < id.length; i++) p += id.charCodeAt(i);
    return (p % 2) === 1 ? 'female' : 'male';
  }

  function heroPortrait(id) {
    var a = S.live[id];
    if (!window.TF_LOOK) return null;
    if (a) {
      var look = null;
      try { look = a.look ? JSON.parse(a.look) : null; } catch (e) { /* lookJson 파손 — 해시 초상 */ }
      var uuid = (look && look.uuid) || id;
      var sex = heroSexOf(id, look);
      var age = look && look.age != null ? Number(look.age) : 40;
      window.TF_LOOK.charSheetUrl({ uuid: uuid, sex: sex, age: age })
        .then(function (u) {
          if (heroPortraits[id] !== u) {
            heroPortraits[id] = u;
            if (S.selected === id) schedule();
          }
        })
        .catch(function () { /* 부품 PNG 로딩 실패 — 초상 없이 렌더 */ });
    }
    // 프레임 공백(재접속·드랍)에도 캐시 초상 유지 — stale 카드의 나머지와 함께
    return heroPortraits[id] || null;
  }

  var HERO_NEEDS = [['needs.hunger', 'hunger'], ['needs.thirst', 'thirst'],
    ['needs.fatigue', 'fatigue'], ['needs.warmth', 'warmth']];

  function heroNeedBar(pair, needs) {
    var v = Number(needs != null && needs[pair[1]] != null ? needs[pair[1]] : 0);
    var pct = Math.min(100, Math.max(0, Math.round(v)));
    return '<div class="tf-hero-need" title="' + esc(t(pair[0])) + '">' +
      '<span class="tf-hero-need-label">' + esc(t(pair[0])) + '</span>' +
      '<div class="tf-bar tf-hero-bar"><div class="tf-bar-fill' + (v > 70 ? ' hot' : '') +
        '" style="width:' + pct + '%"></div></div>' +
      '<span class="tf-hero-need-val' + (v > 70 ? ' hot' : '') + '">' + Math.round(v) + '</span>' +
    '</div>';
  }

  var heroCache = {};  // id → 마지막 스냅숏 — 재접속 공백에도 카드 유지(stale 표시)
  var heroSelPrev = null;  // 직전 선택 id — 변경 시 옛 캐시·초상 정리

  /** 선택 주민 해석 — 라이브 우선, 프레임 공백(재접속·드랍)은 캐시 스냅숏으로
   *  흡수(stale 표시). 선택 해제는 Rust 단일 소스(tf.onPickAgent(null))에 맡긴다. */
  function heroResolve() {
    var a = S.selected ? S.live[S.selected] : null;
    var stale = false;
    if (S.selected !== heroSelPrev) {
      // 선택 해제·전환 — 프레임 공백 흡수 임무는 끝낸 스냅숏·초상 폐기
      delete heroCache[heroSelPrev];
      delete heroPortraits[heroSelPrev];
      heroSelPrev = S.selected;
    }
    if (!a) {
      a = heroCache[S.selected || ''];
      if (!a) return null;
      stale = true; // 프레임 공백 — Rust 유예(5s) 안이면 곧 돌아온다
    } else {
      heroCache[a.id] = a;
    }
    return { a: a, stale: stale };
  }

  function heroBadgeHtml(a) {
    return a.mine
      ? '<span class="tf-hero-badge mine">' + esc(t('mine.badge')) + '</span>'
      : (a.kind === 'bot'
        ? '<span class="tf-hero-badge bot">' + esc(tOr('hero.badge.bot', '봇')) + '</span>'
        : '');
  }

  /** 소지 칩(최대 4종 + 나머지 수) — 뼈대 생성과 값 갱신이 같은 문자열을 쓴다. */
  function heroInvHtml(a) {
    var invKeys = Object.keys(a.inv || {}).filter(function (k) { return (a.inv[k] || 0) > 0; });
    var inv = invKeys.slice(0, 4).map(function (item) {
      return '<span class="tf-chipbox" title="' + esc(tOr('item.' + item, item)) + '">' +
        chipIconHtml(item) + '<span class="tf-qty">×' + a.inv[item] + '</span></span>';
    }).join('');
    if (invKeys.length > 4) inv += '<span class="tf-chipbox more">+' + (invKeys.length - 4) + '</span>';
    return inv;
  }

  /** 카드 뼈대 — 선택·로케일이 바뀔 때에만 다시 쓴다(입장 애니메이션은 그때만).
   *  틱마다 달라지는 값(초상·욕구·소지·행동)은 heroApply가 제자리 갱신 —
   *  노드가 유지돼야 걷기 스프라이트·입장 애니메이션이 매 틱 재시작되지
   *  않는다(깜빡임의 원인). */
  function heroHtml(a, stale, portrait) {
    var inv = heroInvHtml(a);
    var badge = heroBadgeHtml(a);
    var act = a.actType || a.status || '';
    return '<aside class="tf-hero' + (stale ? ' stale' : '') + '" role="region" aria-label="' + esc(tOr('hero.card', '주민 카드')) + '">' +
      '<div class="tf-hero-frame">' +
        '<div class="tf-hero-ground"></div>' +
        (portrait
          ? '<div class="tf-hero-sprite" style="background-image:url(' + portrait + ')"></div>'
          : '<div class="tf-hero-sprite empty">' + svg(ICONS.User, 28) + '</div>') +
      '</div>' +
      '<div class="tf-hero-main">' +
        '<header class="tf-hero-head">' +
          '<span class="tf-hero-name" style="color:' + I.agentColor(a.id) + '">' + esc(a.name) + '</span>' +
          '<span class="tf-hero-badge-slot"' + (badge ? '' : ' style="display:none"') + '>' + badge + '</span>' +
          '<button class="tf-hero-x" data-act="hero-close" title="' + esc(t('btn.close')) + '" aria-label="' + esc(t('btn.close')) + '">' + svg(ICONS.X, 14) + '</button>' +
        '</header>' +
        '<div class="tf-hero-act">' +
          '<span class="tf-hero-act-kind"' + (act ? '' : ' style="display:none"') + '>' +
            (act ? esc(tOr('act.' + act, act)) : '') + '</span>' +
          '<span class="tf-hero-goal"' + (a.goalText ? '' : ' style="display:none"') + '>' +
            (a.goalText ? esc(mt(a.goalText)) : '') + '</span>' +
        '</div>' +
        '<div class="tf-hero-needs">' +
          HERO_NEEDS.map(function (p) { return heroNeedBar(p, a.needs); }).join('') +
        '</div>' +
        '<div class="tf-hero-inv' + (inv ? '' : ' empty') + '">' +
          (inv || '<span class="tf-empty">' + esc(t('panel.inventory.empty')) + '</span>') +
        '</div>' +
        '<div class="tf-hero-btns">' +
          '<button class="tf-hero-btn" data-act="hero-cam" title="' + esc(tOr('hero.cam.hint', '카메라가 이 주민을 따라갑니다')) + '">' +
            svg(ICONS.MapPin, 14) + '<span>' + esc(tOr('hero.cam', '카메라')) + '</span></button>' +
          '<button class="tf-hero-btn" data-act="hero-chat">' +
            svg(ICONS.Users, 14) + '<span>' + esc(tOr('hero.chat', '대화')) + '</span></button>' +
          '<button class="tf-hero-btn" data-act="hero-detail">' +
            svg(ICONS.ScrollText, 14) + '<span>' + esc(tOr('hero.detail', '상세')) + '</span></button>' +
        '</div>' +
      '</div>' +
    '</aside>';
  }

  // 카드 DOM 참조 — 뼈대 재구축(선택·로케일 변경) 시 다시 잡는다
  var heroDom = null;
  function heroUpdate() {
    var r = heroResolve();
    var slot = slots.hero;
    if (!r) {
      if (heroDom) { heroDom = null; slot.innerHTML = ''; }
      return;
    }
    var a = r.a;
    var portrait = heroPortrait(a.id);
    if (!heroDom || heroDom.id !== a.id || heroDom.locale !== I.getLocale() || !slot.contains(heroDom.el)) {
      slot.innerHTML = heroHtml(a, r.stale, portrait);
      var el = slot.firstElementChild;
      heroDom = {
        id: a.id, locale: I.getLocale(), el: el,
        sprite: el.querySelector('.tf-hero-sprite'),
        name: el.querySelector('.tf-hero-name'),
        badge: el.querySelector('.tf-hero-badge-slot'),
        actKind: el.querySelector('.tf-hero-act-kind'),
        goal: el.querySelector('.tf-hero-goal'),
        needs: Array.prototype.map.call(el.querySelectorAll('.tf-hero-need'), function (row) {
          return { fill: row.querySelector('.tf-bar-fill'), val: row.querySelector('.tf-hero-need-val') };
        }),
        inv: el.querySelector('.tf-hero-inv'),
        nameText: null, badgeHtml: null, actText: null, goalShown: null, invHtml: null, portrait: null,
      };
    }
    heroApply(a, r.stale, portrait);
  }

  /** 값 갱신 — 노드를 유지한 채 고치므로 입장·걷기 애니메이션이 이어지고
   *  욕구 바 transition도 자연스럽게 흐른다. */
  function heroApply(a, stale, portrait) {
    var d = heroDom;
    if (!d) return;
    d.el.classList.toggle('stale', stale);
    if (d.portrait !== (portrait || '')) {
      d.portrait = portrait || '';
      if (portrait) {
        d.sprite.classList.remove('empty');
        d.sprite.style.backgroundImage = 'url(' + portrait + ')';
        d.sprite.innerHTML = '';
      } else {
        d.sprite.classList.add('empty');
        d.sprite.style.backgroundImage = '';
        d.sprite.innerHTML = svg(ICONS.User, 28);
      }
    }
    if (d.nameText !== a.name) {
      d.nameText = a.name;
      d.name.textContent = a.name;
      d.name.style.color = I.agentColor(a.id);
    }
    var badge = heroBadgeHtml(a);
    if (d.badgeHtml !== badge) {
      d.badgeHtml = badge;
      d.badge.innerHTML = badge;
      d.badge.style.display = badge ? '' : 'none';
    }
    var act = a.actType || a.status || '';
    if (d.actText !== act) {
      d.actText = act;
      d.actKind.textContent = act ? tOr('act.' + act, act) : '';
      d.actKind.style.display = act ? '' : 'none';
    }
    var goal = a.goalText || '';
    // 원문이 아니라 '화면에 띄울 문장'(mt 캐시 반영)으로 가드 — 번역 완료
    // schedule에서도 교체돼야 한다(원문 가드면 번역이 영원히 안 뜬다).
    var goalShown = goal ? mt(goal) : '';
    if (d.goalShown !== goalShown) {
      d.goalShown = goalShown;
      d.goal.textContent = goalShown;
      d.goal.style.display = goal ? '' : 'none';
    }
    HERO_NEEDS.forEach(function (pair, i) {
      var row = d.needs[i];
      if (!row) return;
      var v = Number(a.needs != null && a.needs[pair[1]] != null ? a.needs[pair[1]] : 0);
      var hot = v > 70;
      var pct = Math.min(100, Math.max(0, Math.round(v)));
      row.fill.style.width = pct + '%';
      row.fill.classList.toggle('hot', hot);
      row.val.textContent = Math.round(v);
      row.val.classList.toggle('hot', hot);
    });
    var inv = heroInvHtml(a);
    if (d.invHtml !== inv) {
      d.invHtml = inv;
      d.inv.innerHTML = inv || '<span class="tf-empty">' + esc(t('panel.inventory.empty')) + '</span>';
      d.inv.classList.toggle('empty', !inv);
    }
  }

  // ── 자원 팝업(FE popups res 포트) ──
    return { html: function () { return charsHtml(); }, heroUpdate: heroUpdate };
  }
  window.TF_CHARS = { create: create };
})();
