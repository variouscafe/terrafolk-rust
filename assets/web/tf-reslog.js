// Terrafolk 자원·기록 패널 — tf-ui.js에서 기능 분리(500줄 규칙, 2026-10-22).
// 자원 팝업(FE popups res 포트 — relic 상세 포함)과 기록(이벤트 로그) 렌더.
// 자원 이용가능 판정(resUsable·resStats — FE civ.ts 포트)과 채팅 로그 표기
// 헬퍼(isHumanRow·isAlreadyKorean·chatDisplayText)도 이 모듈이 소유한다
// (2026-11-03 tf-ui에서 이관 — 이 패널 전용).
// TF_RESLOG.create(ctx) 주입 — tf-ui.js보다 먼저 로드.
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, t = ctx.t, tOr = ctx.tOr, esc = ctx.esc, mt = ctx.mt,
        I = ctx.I, DATA = ctx.DATA, chipIconHtml = ctx.chipIconHtml;

    // ── 자원 이용가능 판정(FE civ.ts 포트) ──
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
  // ── 캐릭터 팝업(FE CharsPopup 포트) ──
  function resHtml() {
    var relic = '';
    if (S.selRelic) {
      relic =
        '<section class="tf-card amber">' +
          '<div style="display:flex;align-items:flex-start;justify-content:space-between;gap:.5rem">' +
            '<h3 class="tf-h-amber"><span>⚒</span> ' + esc(mt(S.selRelic.name)) +
              (S.selRelic.basedOn ? ' <span class="tf-improved">' + esc(t('relic.card.improved', { x: mt(S.selRelic.basedOn) })) + '</span>' : '') +
            '</h3>' +
            '<button class="tf-close-x" data-act="close-relic" aria-label="' + esc(t('btn.close')) + '">✕</button>' +
          '</div>' +
          '<div style="margin-top:.125rem;font-size:10px;color:var(--ink-400)">' +
            esc(t('relic.card.by')) + ' · ' + esc(S.selRelic.by || '—') + '</div>' +
          '<p style="margin:.375rem 0 0;white-space:pre-wrap;font-size:11px;line-height:1.625;color:var(--ink-200)">' +
            (S.selRelic.purpose ? esc(mt(S.selRelic.purpose)) : esc(t('relic.card.desc.empty'))) + '</p>' +
        '</section>';
    }
    var stats = resStats();
    var statsHtml = stats.length
      ? stats.map(function (k) {
          var species = Object.keys(k.species).sort(function (a, b) { return k.species[b].n - k.species[a].n; })
            .map(function (sp) {
              var s = k.species[sp];
              return '<span class="tf-chipbox" title="' + esc(t('res.usable', { n: s.usable })) + '">' +
                esc(tOr('sp.' + sp, sp)) + ' <span class="tf-qty">×' + s.n + '</span></span>';
            }).join('');
          return '<section class="tf-card">' +
            '<div style="display:flex;align-items:center;justify-content:space-between">' +
              '<h3 style="color:var(--ink-200)"><span>' + (I.RES_KIND_ICON[k.kind] || '⛏') + '</span> ' +
                esc(tOr('res.kind.' + k.kind, k.kind)) + '</h3>' +
              '<span style="font-size:10px;color:var(--ink-400)">' +
                esc(t('res.usable', { n: k.usable })) + ' · ' + k.total + '</span>' +
            '</div>' +
            '<div class="tf-species">' + species + '</div>' +
          '</section>';
        }).join('')
      : '<div style="padding:.5rem 0;text-align:center;font-size:12px;color:var(--ink-500)">' + esc(t('res.empty')) + '</div>';

    var invNames = Object.keys(S.inventions);
    var invHtml = invNames.length
      ? invNames.map(function (name) {
          var inv = S.inventions[name];
          return '<span class="tf-chipbox" title="' + esc(inv.inventor ? inv.inventor + ' — ' + mt(inv.purpose || '') : (mt(inv.purpose || '') || '')) + '">' +
            chipIconHtml(name) + esc(mt(name)) +
            (inv.based_on ? '<span class="tf-amber-up" title="' + esc(t('relic.card.improved', { x: mt(inv.based_on) })) + '">↑</span>' : '') +
            '</span>';
        }).join('')
      : '<div class="tf-empty">' + esc(t('res.inventions.empty')) + '</div>';

    return '<div class="tf-res-body tf-scroll" data-scroll="res">' + relic + statsHtml +
      '<section class="tf-card">' +
        '<h3 style="color:var(--ink-200)">' + esc(t('res.inventions')) + '</h3>' +
        '<div class="tf-species">' + invHtml + '</div>' +
      '</section></div>';
  }

  // ── 문명 팝업(FE CivPopup 포트) ──
  // ── 기록 팝업(FE LogFeed 포트) ──
  function logHtml() {
    var rows = S.events.map(function (e) {
      var h = I.highlightOf(e, {
        locale: I.getLocale(),
        tOr: tOr,
        mt: mt,
        nameOf: function (id) { return (S.live[id] && S.live[id].name) || ''; },
      });
      var p = e.payload || {};
      var text = (h && h.text) || I.eventText(p, I.getLocale(), '', mt) || mt(String(p.goal != null ? p.goal : ''));
      var who = (h && h.who) || (e.agent_id ? ((S.live[e.agent_id] && S.live[e.agent_id].name) || '') : '');
      var icon = (h && h.icon) || I.EV_KIND_ICON[e.kind] || '•';
      return { e: e, h: h, text: text, who: who, icon: icon, cat: I.EV_CAT_OF[e.kind] || 'world' };
    }).filter(function (r) { return r.text; }).reverse();
    function inCat(r, c) {
      if (c === 'all') return true;
      if (c === 'highlight') return r.h !== null;
      return r.cat === c;
    }
    var tabs = [{ key: 'all', labelKey: 'panel.log.tab.all' }, { key: 'highlight', labelKey: 'panel.log.tab.highlight' }]
      .concat(I.EV_CATS.map(function (c) { return { key: c, labelKey: 'panel.log.tab.' + c }; }))
      .map(function (tb) {
        var n = rows.filter(function (r) { return inCat(r, tb.key); }).length;
        return '<button class="tf-logtab' + (S.logCat === tb.key ? ' on' : '') + '" data-act="logcat" data-key="' + tb.key + '">' +
          esc(t(tb.labelKey)) + ' <span class="n">' + n + '</span></button>';
      }).join('');
    var shown = rows.filter(function (r) { return inCat(r, S.logCat); });
    var feed =
      '<section class="tf-card" style="display:flex;min-height:180px;flex-direction:column">' +
        '<h3 style="flex-shrink:0">' + esc(t('panel.log')) + '</h3>' +
        '<div class="tf-logtabs">' + tabs + '</div>' +
        '<div class="tf-logrows tf-scroll" data-scroll="log">' +
          (shown.length
            ? shown.map(function (r) {
                return '<div class="tf-logrow">' +
                  '<span class="tf-logrow-icon">' + r.icon + '</span>' +
                  '<span class="tf-logrow-text" style="color:' + (I.EVENT_COLOR[r.e.kind] || 'var(--ink-300)') + '">' +
                    (r.who ? '<span class="tf-logrow-who">' + esc(r.who) + ' — </span>' : '') +
                    esc(r.text) +
                    (r.e.ts ? '<span class="tf-logrow-ts">' + esc(I.fmtTime(r.e.ts)) + '</span>' : '') +
                  '</span></div>';
              }).join('')
            : '<div class="tf-empty">' + esc(t('panel.history.empty')) + '</div>') +
        '</div>' +
      '</section>';

    var chat =
      '<section class="tf-card" style="display:flex;min-height:180px;flex-direction:column">' +
        '<h3 style="flex-shrink:0">' + esc(t('panel.chat')) + '</h3>' +
        '<div class="tf-chatlog tf-scroll" data-scroll="chatlog">' +
          (S.chat.length
            ? S.chat.map(function (c) {
                return '<div class="tf-chatrow"><span class="who' + (isHumanRow(c) ? ' human' : '') + '"' +
                  (isHumanRow(c) ? '' : ' style="color:' + I.agentColor(c.agent_id) + '"') + '>' +
                  esc(c.speaker) + ':</span> <span style="color:var(--ink-300)">' + esc(chatDisplayText(c)) + '</span></div>';
              }).join('')
            : '<div class="tf-empty">' + esc(t('chat.log.empty')) + '</div>') +
        '</div>' +
      '</section>';
    return '<div class="tf-log tf-scroll" data-scroll="logpage">' + feed + chat + '</div>';
  }

  // ── 계정 팝업 ──
    return { html: function () { return resHtml(); }, log: function () { return logHtml(); } };
  }
  window.TF_RESLOG = { create: create };
})();
