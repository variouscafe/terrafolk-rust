// Terrafolk 상단 바·☰ 드롭다운 렌더 — tf-ui.js 분리(500줄 규칙, 2026-11-04).
// 로드 순서 계약: tf-ui.js보다 먼저(다른 패널 모듈과 동일). create(ctx)로
// 조립되며 ctx = { S, t, esc, svg, ICONS, hasToken }.
// ── 상단바: 정보(시계·해시계·날씨·인구)는 bevy_ui 공용 HUD가 담당
// (2026-10-25 통일 2호). DOM은 ☰ 메뉴·모드 칩(관전/참가)·공지만 띄운다.
// ── 드롭다운(2026-11-04 상용화): 별도 고정 슬롯 — topbar 재렌더(공지·
// netLive·scope)에도 열린 메뉴가 재생되지 않는다(애니메이션 재생·포커스
// 유실 방지, 리뷰 11). 모드 헤더(점+라벨)는 모바일에서 레일 칩이 숨겨질
// 때의 유일한 관전/참여 표시(리뷰 8).
(function () {
  'use strict';
  window.TF_TOPBAR = {
    create: function (ctx) {
      var S = ctx.S, t = ctx.t, esc = ctx.esc, svg = ctx.svg;
      var ICONS = ctx.ICONS, hasToken = ctx.hasToken;

      function topbarHtml() {
        var isUser = S.scope && S.scope.kind === 'user';
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
            '</div>' +
            (S.notice
              ? '<div class="tf-notice" data-act="clear-notice">' + esc(S.notice) + '</div>'
              : '') +
          '</div>'
        );
      }

      function menuHtml() {
        if (!S.menuOpen) return '';
        var isUser = S.scope && S.scope.kind === 'user';
        var login = !hasToken();
        var item = function (m) {
          var isLogin = m[0] === 'account' && login;
          return '<button class="tf-menu-item' + (isLogin ? ' login' : '') + '" role="menuitem"' +
            ' data-act="popup" data-key="' + m[0] + '">' +
            '<span class="tf-menu-ico">' + svg(m[2], 15) + '</span>' +
            '<span class="tf-menu-txt">' + esc(m[1]) + '</span></button>';
        };
        return (
          '<div class="tf-menu" role="menu">' +
            '<div class="tf-menu-head">' +
              '<i class="dot' + (S.netLive ? '' : ' off') + '"></i>' +
              '<span>' + esc(isUser ? t('world.joined') : t('world.spectate')) + '</span>' +
            '</div>' +
            [['chars', t('menu.chars'), ICONS.Users],
             ['res', t('menu.resources'), ICONS.TreePine],
             ['inventions', t('menu.civilization'), ICONS.Landmark],
             ['log', t('menu.log'), ICONS.ScrollText]].map(item).join('') +
            '<div class="tf-menu-sep" role="separator"></div>' +
            [['set', t('menu.settings'), ICONS.Settings],
             ['account', login ? t('menu.login') : t('menu.account'), login ? ICONS.LogIn : ICONS.User]]
              .map(item).join('') +
          '</div>'
        );
      }

      return { topbarHtml: topbarHtml, menuHtml: menuHtml };
    },
  };
})();
