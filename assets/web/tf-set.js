// Terrafolk 계정(로그인·로그아웃)·설정(언어·크레딧 진입) 패널 — tf-ui.js에서
// 분리(500줄 규칙, 2026-10-22). TF_SET.create(ctx) 주입 — tf-ui.js보다 먼저 로드.
(function () {
  'use strict';
  function create(ctx) {
    var t = ctx.t, esc = ctx.esc, svg = ctx.svg, ICONS = ctx.ICONS,
        I = ctx.I, hasToken = ctx.hasToken;
  function accountHtml() {
    var body;
    if (hasToken()) {
      body = '<button class="tf-logout" data-act="logout">' + svg(ICONS.LogOut, 14) + ' ' + esc(t('menu.logout')) + '</button>';
    } else {
      body =
        '<p style="margin:0 0 .5rem;font-size:.8rem;color:var(--ink-300)">' +
          esc(t('login.desc')) + '</p>' +
        '<div style="display:flex;flex-direction:column;gap:.25rem">' +
          '<a class="tf-oauth-google" href="/auth/google?next=%2F">' +
            '<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">' +
              '<path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.27-4.74 3.27-8.1Z"/>' +
              '<path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84A11 11 0 0 0 12 23Z"/>' +
              '<path fill="#FBBC05" d="M5.84 14.1a6.6 6.6 0 0 1 0-4.2V7.06H2.18a11 11 0 0 0 0 9.88l3.66-2.84Z"/>' +
              '<path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15A11 11 0 0 0 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52Z"/>' +
            '</svg>' + esc(t('menu.google')) + '</a>' +
          '<a class="tf-oauth-kakao" href="/auth/kakao?next=%2F">' + esc(t('menu.kakao')) + '</a>' +
        '</div>' +
        '<p style="margin:.5rem 0 0;font-size:.75rem;color:var(--ink-300)">' +
          esc(t('login.spectate')) + '</p>';
    }
    return '<div class="tf-res-body tf-scroll"><section class="tf-card">' +
      '<h3>' + esc(t('set.account')) + '</h3>' +
      '<div style="margin-top:.375rem">' + body + '</div>' +
    '</section></div>';
  }

  // ── 설정 팝업 ──
  function setHtml() {
    var locale = I.getLocale();
    var langs = I.LOCALES.map(function (l) {
      return '<button class="tf-lang-btn' + (locale === l.code ? ' on' : '') + '" data-act="locale" data-key="' + l.code + '">' +
        l.flag + ' ' + esc(l.label) + '</button>';
    }).join('');
    return '<div class="tf-res-body tf-scroll">' +
      '<section class="tf-card">' +
        '<h3>' + esc(t('lang.select')) + '</h3>' +
        '<div class="tf-lang-grid">' + langs + '</div>' +
      '</section>' +
      '<section class="tf-card">' +
        '<h3>' + esc(t('set.credits')) + '</h3>' +
        '<button class="tf-credits-btn" data-act="credits">' + svg(ICONS.ScrollText, 14) + ' Terrafolk — ' + esc(t('world.tagline')) + '</button>' +
      '</section>' +
    '</div>';
  }

  // ── 크레딧 모달(FE CreditsPage 포트) ──
    return { account: function () { return accountHtml(); }, html: function () { return setHtml(); } };
  }
  window.TF_SET = { create: create };
})();
