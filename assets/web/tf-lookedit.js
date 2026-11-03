// Terrafolk 외형 편집기(FE LookEditor 포트) — tf-ui.js에서 기능 분리(500줄
// 규칙·기능별 분리, 2026-11-03). 편집 모달 렌더(lookHtml)·에디터 상태
// makeLookEditor(옷 소지 잠금·팔레트 스와치·슬라이드)·look-slide 캡처 리스너를
// 소유한다. 저장(PATCH)·로컬 미리보기 푸시·카메라 포커스는 tf-rest 계층
// hooks(save·previewPush)로 주입받는다. 로드 순서: tf-ui.js보다 앞.
(function () {
  'use strict';
  function create(ctx) {
    var S = ctx.S, t = ctx.t, tOr = ctx.tOr, esc = ctx.esc, svg = ctx.svg,
        ICONS = ctx.ICONS, schedule = ctx.schedule,
        save = ctx.save, previewPush = ctx.previewPush;

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
          swatchRowHtml(t('look.hairColor'), 'hairColor', E.opts.hairColors, E.parts.hairColor || E.opts.hairColors[0], E);
      } else {
        rows = swatchRowHtml(t('look.skin'), 'skin', E.opts.skins, E.parts.skin || E.opts.skins[0], E) +
          swatchRowHtml(t('look.eye'), 'eye', E.opts.eyes, E.parts.eye || E.opts.eyes[0], E) +
          optRowHtml(t('look.beard'), E.opts.beards.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.beard || 'none', E) +
          optRowHtml(t('look.glasses'), E.opts.glasses.map(function (v) { return { key: v, ko: E.prettyPart(v) }; }), E.parts.glasses || 'none', E);
      }
      var tabs = [['clothes', 'look.tab.clothes'], ['hair', 'look.tab.hair'], ['face', 'look.tab.face']].map(function (tb) {
        return '<button class="tf-look-tab' + (tab === tb[0] ? ' on' : '') + '" data-act="look-tab" data-key="' + tb[0] + '">' + esc(t(tb[1])) + '</button>';
      }).join('');
      return '<div class="tf-overlay tf-lookwrap" data-act="close-look">' +
        '<div class="tf-look tf-scroll" data-scroll="look">' +
          '<div class="tf-look-head">' +
            '<div class="tf-look-headgroup">' +
              '<h3 class="tf-look-title">' + esc(t('look.title')) + ' — ' + esc(E.name) + '</h3>' +
              '<span class="tf-look-status' + (E.busy ? ' busy' : '') + '">' +
                (E.busy ? esc(tOr('look.saving', '저장 중…')) : '✓ ' + esc(tOr('look.autosaved', '자동 저장'))) +
              '</span>' +
            '</div>' +
            '<button class="tf-dialog-close" data-act="close-look">' + svg(ICONS.X, 16) + '</button>' +
          '</div>' +
          '<div class="tf-look-body">' +
            '<div class="tf-look-main">' +
              '<div class="tf-look-tabs">' + tabs + '</div>' +
              '<div class="tf-look-rows">' + rows + '</div>' +
              '<p class="tf-look-note">' + esc(tOr('look.preview3d', '위에 보이는 내 캐릭터에 바로 반영돼요 — 옵션을 바꿔보세요')) + '</p>' +
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

    /** 색상 행 — 팔레트 스와치 그리드(2026-10-23 상용 UI): ◀▶ 사이클 대신
     * 전체 팔레트를 나열해 한 번에 보고 탭으로 선택. hex는 3D 틴트 팔레트와
     * 동일(TF_LOOK.swatch) — 고른 색이 미리보기에 그대로 반영된다. */
    function swatchRowHtml(label, kind, options, value, E) {
      var dots = options.map(function (k) {
        var hex = window.TF_LOOK.swatch(kind, k);
        if (!hex) return '';
        return '<button class="tf-pal-dot' + (k === value ? ' on' : '') + '" data-act="look-pick" data-part="' + esc(kind) + '" data-key="' + esc(k) + '" title="' + esc(E.prettyPart(k)) + '" aria-label="' + esc(E.prettyPart(k)) + '" style="background:' + hex + '"></button>';
      }).join('');
      return '<div class="tf-optrow tf-pal">' +
        '<span class="tf-optrow-label">' + esc(label) + '</span>' +
        '<div class="tf-pal-grid">' + dots + '</div>' +
      '</div>';
    }

    /** 옵션 행 — 색 옵션(hairColor·skin·eye)은 3D 틴트 팔레트 스와치, 스타일은
     *  이름만(2D 시트 프리뷰 폐지 — 3D 반영 시대, 실 프리뷰는 저장 후 세계). */
    function optRowHtml(label, options, value, E, swatchKind) {
      var idx = Math.max(0, options.findIndex(function (o) { return o.key === value; }));
      var cur = options[idx] || options[0] || { key: 'none', ko: '' };
      var sw = swatchKind ? window.TF_LOOK.swatch(swatchKind, cur.key) : null;
      var thumb = sw ? '<span class="tf-swatch" style="background:' + sw + '"></span>' : '';
      return '<div class="tf-optrow">' +
        '<span class="tf-optrow-label">' + esc(label) + '</span>' +
        '<button class="tf-optrow-nav" data-act="look-slide" data-dir="-1">' + svg(ICONS.ChevronLeft, 16) + '</button>' +
        '<div class="tf-optrow-main">' +
          thumb +
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
    function makeEditor(m) {
      var opts = window.TF_LOOK.lookPartOptions(m.sex, m.age);
      var base = window.TF_LOOK.buildLook(m.id, m.sex, m.age).picks;
      var E = {
        agentId: m.id, name: m.name, sex: m.sex || 'female', age: m.age || 30,
        opts: opts,
        inv: m.inventory || {}, invClothes: m.clothingInventions || [],
        tab: 'clothes',
        parts: Object.assign({}, base, m.parts || {}),
        wears: Object.assign({}, m.wears || {}),
        busy: false, saveTimer: 0,
        prettyPart: function (v) {
          // 스타일명 한글화(2026-10-23 직접 QA — 'Long' 등 영어 원문 노출).
          // LPC id 마지막 세그먼트를 한국어 라벨로; 미등록은 기존 영문 대문자화.
          var KO = {
            long: '롱헤어', bangs: '앞머리', bun: '번 스타일', afro: '아프로', ponytail: '포니테일',
            bob: '밥컷', buzz: '스포츠', bald: '대머리', balding: '헤어라인', braid: '땋은머리',
            loose: '웨이브', tied: '묶은머리', short: '숏컷', messy: '헝클어진', spiked: '스파이크',
            flat: '플랫', curly: '곱슬', straight: '스트레이트', xlong: '슈퍼롱', pixie: '픽시',
            undercut: '언더컷', mohawk: '모히칸', dreads: '드레드', side: '사이드', top: '탑',
            back: '뒤로넘김', center: '중앙', parted: '가르마', down: '내려쓴', up: '올림',
            beard: '턱수염', mustache: '콧수염', chevron: '쉐브론 콧수염', handlebar: '핸들바',
            lampshade: '램프셰이드', medium: '미디움', trimmed: '다듬은 수염', shadow: '5시 그림자',
            glasses: '기본 안경', nerd: '너드 안경', round: '동그란 안경', halfmoon: '하프문',
            female: '여성용', male: '남성용', plain: '심플', basic: '베이직',
          };
          var s = String(v).split('_').pop() || v;
          // 'none'은 옵션 칩에도 쓰인다(tf-look concat) — 빈 라벨 대신 '없음'.
          if (v === 'none') return '없음';
          if (KO[s]) return KO[s];
          var eng = s.replace(/([a-z])([A-Z])/g, '$1 $2');
          return eng.charAt(0).toUpperCase() + eng.slice(1);
        },
        setPart: function (k, v) { E.parts[k] = v; schedule(); },
        pickWear: function (slot, key) {
          if (key === 'auto') delete E.wears[slot];
          else E.wears[slot] = key;
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
      };
      return E;
    }

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
      // 즉시 미리보기 — 선택 순간 로컬 look 오버라이드로 재스폰(서버 왕복 전).
      // rust(look_edit)가 서버 정본 look에 parts를 머지하고, 저장이 확정되면
      // (룸 lookJson == 오버라이드) 자동 수렴·해제된다. 그림 미리보기 아님.
      previewPush(E);
      // 슬라이드 즉시 저장(0.5s 디바운스) — 실 프리뷰는 투명 오버레이 너머
      // 3D 세계(저장→룸 lookJson→재스폰→틴트). 옷 탭이 아니면 옷 파트 보존.
      clearTimeout(E.saveTimer);
      E.saveTimer = setTimeout(function () { save(tab === 'clothes'); }, 500);
    }, true);

    return { html: lookHtml, makeEditor: makeEditor };
  }
  window.TF_LOOKEDIT = { create: create };
})();
