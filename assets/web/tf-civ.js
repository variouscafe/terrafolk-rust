// Terrafolk 문명(발명품) 패널 — tf-ui.js에서 기능 분리(파일당 500줄·기능별
// 분리 규칙, 2026-10-22). 렌더 로직 전체(시대·그룹 통계·목록·상세·§17.86
// 스프라이트/생김새 서술)를 소유한다. 클로저 의존은 TF_CIV.create(ctx)로
// tf-ui.js가 주입한다. 로드 순서: tf-ui.js와 무관(정의 시점 의존 없음)이나
// index.html 계약상 tf-ui.js 뒤에 둔다.
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, t = ctx.t, tOr = ctx.tOr, esc = ctx.esc, mt = ctx.mt,
        chipIconHtml = ctx.chipIconHtml, I = ctx.I, DATA = ctx.DATA;
  var CIV_GROUPS = ['living', 'production', 'infrastructure', 'culture', 'knowledge', 'institutions'];
  var ERA_STEPS = [
    { key: 'band', at: 0 }, { key: 'camp', at: 6 }, { key: 'village', at: 12 },
    { key: 'large_village', at: 20 }, { key: 'town', at: 32 }, { key: 'city', at: 48 },
    { key: 'metropolis', at: 70 }, { key: 'megacity', at: 100 }, { key: 'beyond', at: 140 },
  ];
  function invCatId(c) {
    var s = String(c == null ? '' : c).trim().toLowerCase();
    return DATA.INVCAT_ALIAS[s] != null ? DATA.INVCAT_ALIAS[s] : s;
  }
  function invGroupOf(c) { return DATA.INVCAT_GROUPS[invCatId(c)] || ''; }
  function invCatEmoji(c) { return DATA.INV_CAT_EMOJI[invCatId(c)] || '⚒️'; }
  function civScore(list) {
    var groups = {};
    var n = 0;
    list.forEach(function (i) { var g = invGroupOf(i.category); if (g && !groups[g]) { groups[g] = 1; n++; } });
    return list.length + n * 2;
  }
  function eraOf(score) {
    var key = 'band';
    ERA_STEPS.forEach(function (s) { if (score >= s.at) key = s.key; });
    return key;
  }
  function civGroupStats(list) {
    return CIV_GROUPS.map(function (group) {
      var cats = {};
      var total = 0;
      list.forEach(function (inv) {
        if (invGroupOf(inv.category) !== group) return;
        var cat = invCatId(inv.category);
        (cats[cat] = cats[cat] || []).push(inv);
        total++;
      });
      return { group: group, total: total, cats: cats };
    });
  }

  // ── 자원 이용가능 판정(FE resUsable 포트) ──

  function civHtml() {
    var all = S.inventionsAll;
    var score = civScore(all);
    var era = eraOf(score);
    var stats = civGroupStats(all);
    var sel = S.selInv ? all.filter(function (i) { return i.name === S.selInv; })[0] : null;

    // 만든 이들 — 발명가별 발명 수(존중 표시층 근사, §17.60)
    var makersMap = {};
    all.forEach(function (i) {
      var w = String(i.inventor || '').trim();
      if (w && w !== 'the ancestors') makersMap[w] = (makersMap[w] || 0) + 1;
    });
    var makers = Object.keys(makersMap).sort(function (a, b) { return makersMap[b] - makersMap[a]; }).slice(0, 4);

    var summary =
      '<section class="tf-card amber">' +
        '<div class="tf-civ-summary-head">' +
          '<h3 class="tf-h-amber">🏛 ' + esc(t('menu.civilization')) + '</h3>' +
          '<span style="font-size:11px;color:var(--ink-300)">' + esc(tOr('era.' + era, era)) + '</span>' +
          '<span style="margin-left:auto;font-size:11px;color:var(--ink-400)">' +
            esc(t('civ.total', { n: all.length })) + ' · ' + esc(t('civ.score', { n: score })) + '</span>' +
        '</div>' +
        '<div class="tf-groups">' +
          stats.map(function (s) {
            var dots = '';
            for (var i = 0; i < 8; i++) dots += '<span class="tf-dot' + (i < Math.min(s.total, 8) ? ' on' : '') + '"></span>';
            return '<div class="tf-group">' +
              '<div class="tf-group-head">' +
                '<span class="tf-group-name">' + esc(tOr('invgroup.' + s.group, s.group)) + '</span>' +
                '<span class="tf-group-n' + (s.total > 0 ? '' : ' zero') + '">' + s.total + '</span>' +
              '</div>' +
              '<div class="tf-dots">' + dots + '</div>' +
            '</div>';
          }).join('') +
        '</div>' +
        (makers.length
          ? '<div class="tf-makers"><span style="color:var(--ink-400)">⚒ ' + esc(t('civ.makers')) + ' ·</span>' +
              makers.map(function (m) {
                return '<span class="tf-maker">' + esc(m) + ' <span style="color:var(--ink-400)">' + makersMap[m] + '</span></span>';
              }).join('') + '</div>'
          : '') +
      '</section>';

    // 살아있는 전통(§17.60) — 문화 계열 발명의 거행 통계
    var living = all.filter(function (i) { return invGroupOf(i.category) === 'culture'; });
    var livingHtml = '';
    if (living.length) {
      var held = living.filter(function (i) { return (i.performs || 0) > 0; });
      livingHtml =
        '<section class="tf-card"><h4>🎏 ' + esc(t('civ.living')) + '</h4>' +
        (held.length === 0
          ? '<div class="tf-empty" style="margin-top:.25rem">' + esc(t('civ.living.empty')) + '</div>'
          : '<div class="tf-species">' + held.map(function (i) {
              var n = i.performs || 0;
              var depth = Math.min(3, Math.floor(n / 4));
              return '<span class="tf-chipbox"' +
                (i.last_perform_day != null ? ' title="' + esc(t('civ.living.last', { d: i.last_perform_day })) + '"' : '') + '>' +
                '<span style="font-size:14px;line-height:1">' + (i.icon && i.icon.tpl ? chipIconHtml(i.name, 20) : invCatEmoji(i.category)) + '</span>' +
                esc(i.name) +
                '<span class="tf-qty" style="color:#fcd34d">' + esc(t('civ.living.holds', { n: n, d: depth })) + '</span>' +
                '</span>';
            }).join('') + '</div>') +
        '</section>';
    }

    // 마을 연대기 — kind='chronicle' 최근 12행(최신 우선)
    var chron = S.events.filter(function (e) { return e.kind === 'chronicle'; })
      .map(function (e) { return I.eventText(e.payload || {}, I.getLocale()); })
      .filter(Boolean).slice(-12).reverse();
    var chronHtml =
      '<section class="tf-card"><h4>📜 ' + esc(t('civ.story')) + '</h4>' +
      (chron.length === 0
        ? '<div class="tf-empty" style="margin-top:.25rem">' + esc(t('civ.story.empty')) + '</div>'
        : '<ol style="margin:.375rem 0 0;padding-left:1.25rem;display:flex;flex-direction:column;gap:.25rem;font-size:11px;line-height:1.625;color:var(--ink-300)">' +
            chron.map(function (r) { return '<li style="color:rgba(254,243,199,.9)">' + esc(r) + '</li>'; }).join('') + '</ol>') +
      '</section>';

    var listHtml;
    if (all.length === 0) {
      listHtml = '<div style="padding:1.5rem 0;text-align:center;font-size:12px;color:var(--ink-500)">' +
        esc(t('res.inventions.empty')) + '</div>';
    } else if (sel) {
      listHtml = invDetailHtml(sel);
    } else {
      listHtml = stats.filter(function (s) { return s.total > 0; }).map(function (s) {
        var rows = [];
        Object.keys(s.cats).forEach(function (cat) { rows = rows.concat(s.cats[cat]); });
        rows.sort(function (a, b) { return (b.day || 0) - (a.day || 0); });
        return '<section class="tf-card">' +
          '<h4>' + esc(tOr('invgroup.' + s.group, s.group)) +
            ' <span style="margin-left:.375rem;font-weight:400;color:var(--ink-500)">(' + s.total + ')</span></h4>' +
          '<div class="tf-invrows">' + rows.map(function (i) { return invRowHtml(i); }).join('') + '</div>' +
        '</section>';
      }).join('');
    }

    return '<div class="tf-civ tf-scroll" data-scroll="civ">' + summary + livingHtml + chronHtml + listHtml + '</div>';
  }

  function invIconHtml(i, px) {
    if (i.icon && i.icon.tpl) return chipIconHtml(i.name, px);
    return '<span style="font-size:' + (px >= 24 ? 24 : 16) + 'px;line-height:1">' +
      (I.INV_SEED_ICON[i.name] || invCatEmoji(i.category)) + '</span>';
  }
  function invRowHtml(i) {
    var year = I.dateOf(i.day || 0).year;
    var cat = invCatId(i.category);
    return '<button class="tf-invrow" data-act="sel-inv" data-name="' + esc(i.name) + '">' +
      '<span class="tf-invrow-icon">' + invIconHtml(i, 22) + '</span>' +
      '<span class="tf-invrow-main">' +
        '<span class="tf-invrow-line1">' +
          '<span class="tf-invrow-name">' + esc(i.name) + '</span>' +
          (i.based_on ? '<span class="tf-amber-up" title="' + esc(t('relic.card.improved', { x: i.based_on })) + '">↑</span>' : '') +
          '<span class="tf-invrow-year">' + esc(t('civ.inv.year', { y: year })) + '</span>' +
        '</span>' +
        '<span class="tf-invrow-line2">' + invCatEmoji(i.category) + ' ' + esc(tOr('invcat.' + cat, cat || '—')) +
          ' · ' + esc(t('relic.card.by')) + ' ' + esc(i.inventor || '—') + '</span>' +
        (i.purpose ? '<span class="tf-invrow-desc">' + esc(mt(i.purpose)) + '</span>' : '') +
      '</span>' +
    '</button>';
  }
  function invDetailHtml(sel) {
    var cal = I.dateOf(sel.day || 0);
    var mat = sel.materials
      ? Object.keys(sel.materials).map(function (it) { return it + '×' + sel.materials[it]; }).join(' · ')
      : '';
    var cat = invCatId(sel.category);
    // §17.86 생성 스프라이트 — 핸드페인트 래스터: 부드러운 샘플링·contain.
    // 폴백 체인: sprite → 로드 실패 시 onerror로 숨김(헤더 아이콘 유지) → icon.
    var spriteHtml = sel.sprite
      ? '<div class="tf-invdetail-sprite" style="display:flex;justify-content:center;margin:.5rem 0 .25rem">' +
          '<img src="' + esc(sel.sprite) + '" alt="" loading="lazy" ' +
            'style="max-height:150px;max-width:100%;object-fit:contain" ' +
            'onerror="this.parentNode.style.display=\'none\'">' +
        '</div>'
      : '';
    // 발명가의 생김새 서술(§17.86 design) — 도감 상세 표시용 문장.
    var designHtml = sel.design
      ? '<p class="tf-invdetail-design" style="margin:.375rem 0 0;font-size:11px;line-height:1.6;color:var(--ink-300);font-style:italic">“' +
          esc(mt(sel.design)) + '”</p>'
      : '';
    return '<section class="tf-card amber tf-invdetail">' +
      '<div class="tf-invdetail-head">' +
        '<div class="tf-invdetail-icon">' + invIconHtml(sel, 32) + '</div>' +
        '<div style="min-width:0;flex:1 1 0%">' +
          '<div style="display:flex;align-items:center;gap:.375rem">' +
            '<h3 class="tf-invdetail-name">' + esc(sel.name) + '</h3>' +
            (sel.based_on ? '<span class="tf-improved">' + esc(t('relic.card.improved', { x: sel.based_on })) + '</span>' : '') +
          '</div>' +
          '<div class="tf-invdetail-meta">' +
            '<span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + esc(t('relic.card.by')) + ' · ' + esc(sel.inventor || '—') + '</span>' +
            '<span class="tf-invdetail-date">' + esc(t('civ.inv.date', { y: cal.year, m: cal.month, d: cal.dayOfMonth })) + '</span>' +
            '<span class="tf-invdetail-cat">' + esc(tOr('invcat.' + cat, cat || '—')) + '</span>' +
          '</div>' +
        '</div>' +
        '<button class="tf-close-x" data-act="close-inv" aria-label="' + esc(t('ui.back')) + '">✕</button>' +
      '</div>' +
      (mat ? '<div class="tf-invdetail-mat"><span style="color:var(--ink-400)">' + esc(t('inv.materials')) + ' · </span>' + esc(mat) + '</div>' : '') +
      spriteHtml +
      '<p class="tf-invdetail-purpose">' +
        (sel.purpose ? esc(mt(sel.purpose)) : esc(t('relic.card.desc.empty'))) + '</p>' +
      designHtml +
    '</section>';
  }

    return { html: function () { return civHtml(); } };
  }
  window.TF_CIV = { create: create };
})();
