// Terrafolk 크레딧 화면 — tf-ui.js에서 기능 분리(500줄 규칙·기능별 분리,
// 2026-11-03). LPC 저작자 전체 목록(GPL-3.0/CC-BY-SA 이행)·Kenney(CC0)·
// game-icons(Lorc·Delapouite) 표기 렌더. 데이터(DATA.CREDITS)는
// tools/gen-web-data.mjs가 FE 정본(../terrafolk-2d-legacy)에서 생성.
// TF_CREDITS.create(ctx) 주입 — tf-ui.js보다 먼저 로드.
(function () {
  'use strict';
  function create(ctx) {
    var t = ctx.t, esc = ctx.esc, svg = ctx.svg, ICONS = ctx.ICONS, DATA = ctx.DATA;

    function html() {
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

    return { html: html };
  }
  window.TF_CREDITS = { create: create };
})();
