# 3D 캐릭터 커스터마이징 — 현황과 확장 로드맵

작성 2026-11-03(0.8.55 기준). 질문 "살 찌우기·팔 퉁퉁·눈 커스텀, 완전 다
커스텀 가능한 상태야?"에 대한 답과 다음 세션 착수 지점을 남긴다.

## 현재 가능 (런타임)

| 항목 | 상태 | 구현 |
|---|---|---|
| 피부·머리색·**눈 색** | ✔ | 재질 틴트 — `world/look_tint.rs`(parts.{skin,eye,hairColor,topColor,bottomColor,feetColor}) |
| 헤어 스타일 | ✔ 2종 GLB | 본 앵커 부착 — `world/wear.rs` |
| 수염 | ✔ 7종 | 〃 |
| 안경 | ✔ 4종 | 〃 |
| 옷(상의/하의/신발) | ✔ 소지 기반 잠금 | wears 파생 + 색 패치(FE LookEditor 규약 — 옷 탭 저장 시 parts 옷 키 삭제 후 wears 송신) |
| 발명 의상 | ✔ | clothingInventions(색 커스텀 창작 옷) |
| 편집 UI | ✔ | 웹 tf-lookedit.js · 네이티브 ui/look.rs·look_model.rs·look_view.rs(3탭: 옷·헤어·얼굴) |

## 현재 불가

- **몸매 비율**(살 찌움·팔 퉁퉁·키·근육) — 몸 베이스는 Quaternius GLB 6종
  (남 4·여 2, `world/villager.rs` MODELS_M/F)이 고정 체형, id 해시 자동 배정.
- **눈 모양** — 색 틴트만. CHIBI_REF의 "큰 눈"(`world/reference_look.rs`)은
  전 주민 공통 고정 룩이라 개인 커스텀이 아님.
- 얼굴형·임의 형태 변경 전부 불가.

## 아키텍처 사실 — 절반은 이미 와 있다

- `chibi-character` 크레이트 `proportion.rs`가 **본(bone) 단위 런타임 스케일
  변형**으로 치비화를 구현한다(원본 리그 ≈6두신 → 카툰 비율, 본마다
  `bone_scale` 테이블). "팔 퉁퉁"의 기계장치(뼈 스케일)가 이미 존재하며,
  **고정 프로파일 1종**이라는 게 한계다.
- ⚠️ 이 영역은 발 착지 버그의 역사(발 스케일 fix 브랜치 11개 —
  docs/character-module-redesign.md §1 "패치가 패치를 낳는 구조")가 있는
  곳. 체형 추가마다 **접지 게이트(feet_check·grav_drop) + 치비 파머
  라인 육안 검증이 필수**다.

## 확장 경로 3단계

### 1단계 — 체형 프리셋 3~5종 (짧은 길, 권장 · 다음 착수 지점)

치비 프로파일을 파라미터화해 **마름/보통/통통/근육** 중 선택, lookJson에 저장.

구현 지점 (**오픈소스 레포 먼저 → 게임 적용** — CLAUDE.md 역할 분담 계약):
- ① `../3d-chibi-character/crates/chibi-character/src/proportion.rs` —
  `bone_scale` 프로파일을 enum/파라미터로. **크레이트에서 먼저 구현·검증**:
  순수성 게이트 tools/check-purity.sh + `/character` 뷰어(viewer_shot.cjs)로
  프리셋별 실루엣 확인.
- ② 크레이트 버전 bump → 게임에 "캐릭터 크레이트 0.2.x 반영" 커밋
  (0.8.46~0.8.50 패턴). ⚠️ 게임 main은 크레이트 **main=09a46d9(0.2.6)** 페어
  (2026-11-03 — folk 0.4.1 라인은 태그 archive/folk-0.4.1, API 불일치).
- ③ lookJson 새 필드(예: `build`: "slim"|"normal"|"chubby"|"sturdy") —
  계약 문서(../terrafolk docs/data-contract.md §17.32) 보강 동반.
- ④ `world/look_tint.rs` 재스폰 트리거에 build 포함(res_sig류 판정).
- ⑤ 편집기: 웹 tf-lookedit.js + 네이티브 ui/look.rs 슬라이더 1줄 추가.
- 검증: feet_check·grav_drop·root3d_live_villagers + 통통 프로파일에서
  발 부양/침투 육안 스윕.

### 2단계 — 부위별 슬라이더 (중간)

팔/다리/몸통/머리 크기 개별 조정(본 스케일 런타임 파라미터화).
접지(`foot_lock`)·다리 IK(`leg_ik`) 재셋틀이 필요해 게이트 강화 동반.
1단계 프리셋이 안정된 뒤만.

### 3단계 — 완전 커스텀 (긴 길)

눈 모양·얼굴형 = **blend shape(shape key) 파이프라인 선행** 필요.
`../terrafolk-character`(bpy 휠)에서 GLB 생성 시 셰이프키를 심는 확장
→ 런타임 모프 가중치 = lookJson. 현재 눈은 텍스처/디테일 수준이라
메시 개조부터.

## 참고

- 외형 편집 전 경로(저장 계약·낙관적 재스폰): docs/parity-3d.md,
  `world/look_edit.rs` 헤더.
- 지오메트리 축 1차(헤어 GLB·수염·안경·봇 uuid 룩): CLAUDE.md
  '외형 편집 3D 반영' 절 — 헤어 N종 확장 유보(LPC 카드 각도 문제) 기록.
