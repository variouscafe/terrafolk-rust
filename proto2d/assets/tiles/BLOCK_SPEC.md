# 지형 블록 타일 정준 규격 (mewki 팩 기준)

- 출처: mewki 「Isometric Nature Blocks」 (itch.io, 상업적 사용 허가·재배포 금지)
- 캔버스: **256×312** — 상판 다이아몬드 **256×200** (비율 0.78125), 상단정점
  **(128, 16)**, 중심 (128, 116)
- **에지 블리드 2px** — 불투명 마스크 팽창+가장자리 색 복제. 앞 타일(y-sort)
  이 시임을 겹쳐 덮어 AA 시임·벽 노출을 봉쇄한다 (2026-10-19 '계단 시임' 수정)
- 월드: TW=128, TH=100(=캔버스 200px 피치와 정합), 스프라이트 오프셋 (0, −20),
  z=py·0.001 (y-sort)
- **물 평면 낙차 12 월드** — 해안에서 육지 벽이 절벽으로 드러난다(섬 슬래브).
  내부 육지는 하나의 이어진 평면
- 변형: GID별 water 4(애님)·sand 4·grass 4·**forest 3**(forest_3은 큰 나무 장식
  — 블럭 기하가 없어 평면 계약 밖, 향후 props 소재)·rock 4·dirt 4·**snow 4**
  (계절 winter·날씨 snow 때 육지 GID 전부에 적용 — 눈담요, 물은 유지.
  하이브리드: ComfyUI 텍스처 상판 × Tier1 측벽, assetgen `core/snowtile.py`)
- water·sand: assetgen 서비스가 동일 규격으로 생성
- 재생성: `python3 tools/painted/apply_mewki_blocks.py <팩 디렉터리>` →
  `python3 tools/painted/normalize_block_tiles.py` (스냅+블리드+게이트)·
  snow는 assetgen `python3 -m core.snowtile --out <이 디렉터리>`
- **미지 GID 파이프라인**(어휘 확장 대비 — 현재 6종 하드캡): 라이브 맵에
  미지 GID가 오면 런타임은 잔디+결정론 색조 폴백으로 관전 유지(ground.rs).
  대응 절차: `TF_DUMP_GIDS=1` 실행으로 히스토그램 확인 → assetgen에 팔레트
  (style.json)+페인터 추가·생성 → `normalize_block_tiles.py --only 'iso_<t>_*'`
  게이트 → 커밈. **--only 필수**(기존 타일 재스냅 수축 방지)
