//! 와이어 디코더 검증 — 합성 바이트 fixture(프로토콜 조사 정본의 worked
//! example 기반). 실서버 없이 오퍼레이션 문법·refId 스위치·맵 DELETE·
//! reflection 부트스트랩을 확인한다.

use colyseus_lite::schema::decode::install_types;
use colyseus_lite::schema::{
    FieldChild, FieldDef, FieldKind, Primitive, SchemaState, TypeDef, Value,
};

// ── 바이트 조립 헬퍼(notepack LE 방언) ──

/// notepack 숫자 — 작은 정수는 fixint, 나머지는 f64(LE).
fn num(v: f64) -> Vec<u8> {
    if v.fract() == 0.0 && (-32.0..=127.0).contains(&v) {
        vec![v as i8 as u8]
    } else {
        let mut b = vec![0xCB];
        b.extend_from_slice(&v.to_le_bytes());
        b
    }
}

/// notepack 문자열 — fixstr/0xDA(LE 16bit 길이).
fn str_le(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    match b.len() {
        0..=31 => out.push(0xA0 | b.len() as u8),
        _ => {
            out.push(0xDA);
            out.extend_from_slice(&(b.len() as u16).to_le_bytes());
        }
    }
    out.extend_from_slice(b);
    out
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

// ── 미니 스키마: World(0) { tick u32, label str, temp num, agents map<Agent> }
//    Agent(1) { name str, x num } ──

fn mini_types() -> Vec<TypeDef> {
    vec![
        TypeDef {
            id: 0,
            extends: None,
            fields: vec![
                FieldDef {
                    name: "tick".into(),
                    kind: FieldKind::Primitive(Primitive::U32),
                },
                FieldDef {
                    name: "label".into(),
                    kind: FieldKind::Primitive(Primitive::String),
                },
                FieldDef {
                    name: "temp".into(),
                    kind: FieldKind::Primitive(Primitive::Number),
                },
                FieldDef {
                    name: "agents".into(),
                    kind: FieldKind::MapOf(FieldChild::Ref(1)),
                },
            ],
        },
        TypeDef {
            id: 1,
            extends: None,
            fields: vec![
                FieldDef {
                    name: "name".into(),
                    kind: FieldKind::Primitive(Primitive::String),
                },
                FieldDef {
                    name: "x".into(),
                    kind: FieldKind::Primitive(Primitive::Number),
                },
            ],
        },
    ]
}

fn mini_state() -> SchemaState {
    let mut s = SchemaState::new();
    install_types(&mut s, 0, mini_types());
    s
}

/// 미니 스키마 전체 상태 바이트 — 루트 옵은 스위치 없이 곧장(정본 규약).
fn mini_full() -> Vec<u8> {
    let tick: u32 = 4242;
    cat(&[
        &[0x80],             // ADD | 필드0 tick
        &tick.to_le_bytes(), // u32 원시 LE
        &[0x81],             // ADD | 필드1 label
        &str_le("마을"),
        &[0x82], // ADD | 필드2 temp
        &num(18.5),
        &[0x83], // ADD | 필드3 agents → 맵 refId 1
        &num(1.0),
        &[0xFF, 0x01],    // 스위치 → 맵 1
        &[0x80],          // 맵 ADD
        &num(0.0),        // 와이어 인덱스 0
        &str_le("bot-1"), // 키
        &num(2.0),        // 자식 Agent refId 2
        &[0xFF, 0x02],    // 스위치 → Agent 2
        &[0x80],          // ADD | 필드0 name
        &str_le("아다"),
        &[0x81], // ADD | 필드1 x
        &num(10.5),
    ])
}

#[test]
fn full_state_decodes() {
    let mut s = mini_state();
    s.apply_full(&mini_full()).expect("전체 상태 디코딩");
    let root = s.root();
    assert_eq!(root.field("tick").and_then(Value::as_f64), Some(4242.0));
    assert_eq!(root.field("label").and_then(Value::as_str), Some("마을"));
    assert_eq!(root.field("temp").and_then(Value::as_f64), Some(18.5));
    let agents = root.field("agents").expect("agents");
    let entries = agents.entries().expect("맵");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "bot-1");
    let ada = entries[0].1.field("name").and_then(Value::as_str);
    assert_eq!(ada, Some("아다"));
    assert_eq!(entries[0].1.field("x").and_then(Value::as_f64), Some(10.5));
}

#[test]
fn patch_replace_add_delete() {
    let mut s = mini_state();
    s.apply_full(&mini_full()).expect("전체 상태");
    // 패치: temp 교체(루트 REPLACE), bot-1.x 교체, bot-2 추가, bot-1 삭제.
    let patch = cat(&[
        &[0x02], // REPLACE | 필드2 temp
        &num(21.0),
        &[0xFF, 0x01], // 스위치 → agents 맵
        &[0x80],       // 맵 ADD(신규 bot-2)
        &num(1.0),
        &str_le("bot-2"),
        &num(3.0), // Agent refId 3
        &[0xFF, 0x03],
        &[0x80],
        &str_le("벨"),
        &[0x81],
        &num(30.25),
        &[0xFF, 0x01], // 스위치 → 맵
        &[0x40],       // 맵 DELETE — 와이어 인덱스만
        &num(0.0),     // wire 0 = bot-1
        &[0xFF, 0x02], // 스위치 → Agent 2(bot-1)
        &[0x01],       // REPLACE | 필드1 x
        &num(99.0),
    ]);
    s.apply_patch(&patch).expect("패치");
    let root = s.root();
    assert_eq!(root.field("temp").and_then(Value::as_f64), Some(21.0));
    let entries = root.field("agents").expect("agents").entries().expect("맵");
    assert_eq!(entries.len(), 1, "bot-1 삭제 후 bot-2만");
    assert_eq!(entries[0].0, "bot-2");
    assert_eq!(
        entries[0].1.field("name").and_then(Value::as_str),
        Some("벨")
    );
    assert_eq!(entries[0].1.field("x").and_then(Value::as_f64), Some(30.25));
    // 삭제된 인스턴스의 교체는 고아 — 루트 뷰에 영향 없음(크래시·에러 없이)
}

#[test]
fn delete_and_add_rekeys_entry() {
    let mut s = mini_state();
    s.apply_full(&mini_full()).expect("전체 상태");
    // DELETE_AND_ADD(192): 같은 와이어 인덱스를 새 키로 교체.
    let patch = cat(&[
        &[0xFF, 0x01],
        &[0xC0], // DELETE_AND_ADD
        &num(0.0),
        &str_le("bot-9"),
        &num(2.0), // 기존 Agent 2 재사용
    ]);
    s.apply_patch(&patch).expect("패치");
    let entries = s
        .root()
        .field("agents")
        .expect("agents")
        .entries()
        .expect("맵");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "bot-9");
    assert_eq!(
        entries[0].1.field("name").and_then(Value::as_str),
        Some("아다")
    );
}

#[test]
fn bool_and_f32_primitives() {
    let mut s = SchemaState::new();
    install_types(
        &mut s,
        0,
        vec![TypeDef {
            id: 0,
            extends: None,
            fields: vec![
                FieldDef {
                    name: "lit".into(),
                    kind: FieldKind::Primitive(Primitive::Bool),
                },
                FieldDef {
                    name: "f".into(),
                    kind: FieldKind::Primitive(Primitive::F32),
                },
                FieldDef {
                    name: "u16v".into(),
                    kind: FieldKind::Primitive(Primitive::U16),
                },
            ],
        }],
    );
    let bytes = cat(&[
        &[0x80, 0x01], // lit = true(원시 1바이트)
        &[0x81],       // f = f32 원시 LE
        &2.5f32.to_le_bytes(),
        &[0x82], // u16v 원시 LE
        &600u16.to_le_bytes(),
    ]);
    s.apply_full(&bytes).expect("디코딩");
    let root = s.root();
    assert_eq!(root.field("lit").and_then(Value::as_bool), Some(true));
    assert_eq!(root.field("f").and_then(Value::as_f64), Some(2.5));
    assert_eq!(root.field("u16v").and_then(Value::as_f64), Some(600.0));
}

// ── reflection 부트스트랩: 미니 스키마의 reflection을 손으로 인코딩 ──

fn reflection_bytes() -> Vec<u8> {
    // Reflection{id:0} 필드: [0] types 배열<Ref>, [1] rootType num
    // 항목 타입 id=0(World), 1(Agent) — 와이어 타입 id와 일치.
    fn field(name: &str, ty: &str, refd: f64, child: &str) -> Vec<u8> {
        cat(&[
            &[0x80],
            &str_le(name),
            &[0x81],
            &str_le(ty),
            &[0x82],
            &num(refd),
            &[0x83],
            &str_le(child),
        ])
    }
    cat(&[
        &[0x80, 0x01], // types → 배열 refId 1
        &[0x81],       // rootType = 0
        &num(0.0),
        &[0xFF, 0x01], // 스위치 → 배열
        &[0x80, 0x00], // [0] = 타입 항목 refId 2
        &num(2.0),
        &[0x80, 0x01], // [1] = 타입 항목 refId 3
        &num(3.0),
        // World(2): id 0, extendsId -1, fields → 배열 4
        &[0xFF, 0x02],
        &[0x80],
        &num(0.0), // id
        &[0x81],
        &num(-1.0),    // extendsId
        &[0x82, 0x04], // fields → 배열 refId 4 (fixint 4)
        &[0xFF, 0x04],
        &[0x80, 0x00], // [0] = 필드 refId 5
        &num(5.0),
        &[0x80, 0x01], // [1] = 필드 refId 6
        &num(6.0),
        &[0x80, 0x02], // [2] = 필드 refId 7
        &num(7.0),
        &[0x80, 0x03], // [3] = 필드 refId 8
        &num(8.0),
        &[0xFF, 0x05],
        &field("tick", "uint32", -1.0, ""),
        &[0xFF, 0x06],
        &field("label", "string", -1.0, ""),
        &[0xFF, 0x07],
        &field("temp", "number", -1.0, ""),
        &[0xFF, 0x08],
        &field("agents", "map", 1.0, ""),
        // Agent(3): id 1, extendsId -1, fields → 배열 9
        &[0xFF, 0x03],
        &[0x80],
        &num(1.0),
        &[0x81],
        &num(-1.0),
        &[0x82, 0x09],
        &[0xFF, 0x09],
        &[0x80, 0x00],
        &num(10.0), // 필드 refId 10
        &[0x80, 0x01],
        &num(11.0), // 필드 refId 11
        &[0xFF, 10],
        &field("name", "string", -1.0, ""),
        &[0xFF, 11],
        &field("x", "number", -1.0, ""),
    ])
}

#[test]
fn reflection_bootstraps_schema() {
    let mut s = SchemaState::new();
    s.apply_reflection(&reflection_bytes()).expect("reflection");
    // 발견된 스키마로 실제 상태 디코딩 — 미니 전체 상태 재사용.
    s.apply_full(&mini_full()).expect("전체 상태");
    assert_eq!(s.root().field("tick").and_then(Value::as_f64), Some(4242.0));
    let ada = s
        .root()
        .field("agents")
        .and_then(|a| a.entries())
        .and_then(|e| e.first())
        .and_then(|(_, v)| v.field("name"))
        .and_then(Value::as_str);
    assert_eq!(ada, Some("아다"));
}

// ── 프로토콜 프레임 ──

#[test]
fn join_room_frame_parses() {
    let reflection = [0x80u8, 0x01, 0x42];
    let mut body = vec![3u8];
    body.extend_from_slice(b"rct");
    body.push(6);
    body.extend_from_slice(b"schema");
    body.push(0x03); // fixint 3
    body.extend_from_slice(&reflection);
    let info = colyseus_lite::protocol::parse_join_room(&body).expect("조인 파싱");
    assert_eq!(info.reconnection_token, "rct");
    assert_eq!(info.serializer_id, "schema");
    assert_eq!(info.reflection, reflection.to_vec());
}

#[test]
fn msgpack_be_round_trip() {
    use colyseus_lite::msgpack;
    let v = serde_json::json!({
        "agentId": "bot-1",
        "text": "안녕!",
        "n": 42,
        "f": 1.5,
        "flag": true,
        "nil": null,
        "arr": [1, "two", false],
    });
    let mut buf = Vec::new();
    msgpack::write_value(&mut buf, &v);
    let mut off = 0usize;
    let back = msgpack::read_value(&buf, &mut off).expect("역직렬화");
    assert_eq!(off, buf.len(), "전량 소비");
    assert_eq!(back, v);
}

#[test]
fn ws_url_builds() {
    assert_eq!(
        colyseus_lite::ws_url("https://tf-world.example", "p1", "r1", "s1"),
        "wss://tf-world.example/p1/r1?sessionId=s1"
    );
    assert_eq!(
        colyseus_lite::ws_url("http://127.0.0.1:8893", "p", "r", "s"),
        "ws://127.0.0.1:8893/p/r?sessionId=s"
    );
}
