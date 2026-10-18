//! @colyseus/schema 5.x 상태 — 동적(코드젠 불필요) 트리.
//!
//! 서버가 조인 시 보내는 Reflection 바이트에서 타입 정의(필드 인덱스·종류)를
//! 발견해 임의 스키마를 디코딩한다. 앱은 `root()` 값 트리를 필드 이름으로
//! 탐색한다 — 특정 스키마 지식이 라이브러리에 없다(게임 무관 재사용).

pub mod decode;

use std::collections::HashMap;
use std::sync::Arc;

/// 원시 필드 타입 — 인코딩이 3갈래임에 주의(스키마 와이어의 함정):
/// `Number`/`String`은 msgpack(notepack LE), `Bool`은 원시 1바이트,
/// 나머지 고정폭 정수·float은 **원시 LE 바이트**(msgprepix 없음).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Bool,
    Number,
    String,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Primitive {
    fn parse(name: &str) -> Option<Primitive> {
        Some(match name {
            "boolean" => Primitive::Bool,
            "number" => Primitive::Number,
            "string" => Primitive::String,
            "int8" => Primitive::I8,
            "uint8" => Primitive::U8,
            "int16" => Primitive::I16,
            "uint16" => Primitive::U16,
            "int32" => Primitive::I32,
            "uint32" => Primitive::U32,
            "int64" => Primitive::I64,
            "uint64" => Primitive::U64,
            "float32" => Primitive::F32,
            "float64" => Primitive::F64,
            _ => return None,
        })
    }
}

/// 컬렉션 원소 종류 — 원시 또는 다른 구조체 참조.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldChild {
    Primitive(Primitive),
    Ref(u16),
}

/// 필드 종류.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldKind {
    Primitive(Primitive),
    /// 다른 구조체 인스턴스(와이어: msgpack refId).
    Ref(u16),
    MapOf(FieldChild),
    ArrayOf(FieldChild),
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub kind: FieldKind,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub id: u16,
    pub extends: Option<u16>,
    pub fields: Vec<FieldDef>,
}

/// 디코딩 결과 값 — 구조체 인스턴스는 필드 이름 → 값의 `Map`.
/// `Ref`는 내부 자리표(아레나 노드) — `root()` 반환 시엔 항상 풀려있다.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Map(Vec<(String, Value)>),
    Ref(u32),
}

impl Value {
    pub fn field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    /// 맵 엔트리(구조체 필드·MapSchema 공용).
    pub fn entries(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }
    pub fn items(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(items) => Some(items),
            _ => None,
        }
    }
}

/// 아레나 노드 — refId로 참조되는 디코딩 단위.
pub(crate) enum RefNode {
    Struct {
        def: Arc<TypeDef>,
        fields: Vec<Option<Value>>,
    },
    Map {
        child: FieldChild,
        /// 와이어 인덱스 → 문자열 키(DELETE 연산 해석용).
        index_key: Vec<(u64, String)>,
        entries: Vec<(String, Value)>,
    },
    Array {
        child: FieldChild,
        items: Vec<Value>,
    },
}

/// 스키마 상태 — reflection으로 타입 발견 → 전체 상태/패치 적용 → 값 트리.
#[derive(Default)]
pub struct SchemaState {
    pub(crate) types: HashMap<u16, Arc<TypeDef>>,
    pub(crate) root_type: u16,
    pub(crate) arena: HashMap<u32, RefNode>,
    view: Value,
}

impl SchemaState {
    pub fn new() -> Self {
        Self::default()
    }

    /// JOIN_ROOM 핸드셰이크(Reflection 바이트)로 스키마 발견.
    /// 이후 `apply_full`/`apply_patch`를 쓸 수 있다.
    pub fn apply_reflection(&mut self, bytes: &[u8]) -> Result<(), String> {
        let mut seed = SchemaState {
            types: HashMap::new(),
            root_type: REFLECTION_TYPE,
            arena: HashMap::new(),
            view: Value::Nil,
        };
        let put = |types: &mut HashMap<u16, Arc<TypeDef>>, def: TypeDef| {
            let id = def.id;
            types.insert(id, Arc::new(def));
        };
        put(
            &mut seed.types,
            TypeDef {
                id: REFLECTION_TYPE,
                extends: None,
                fields: vec![
                    FieldDef {
                        name: "types".into(),
                        kind: FieldKind::ArrayOf(FieldChild::Ref(REFLECTION_ITEM)),
                    },
                    FieldDef {
                        name: "rootType".into(),
                        kind: FieldKind::Primitive(Primitive::Number),
                    },
                ],
            },
        );
        put(
            &mut seed.types,
            TypeDef {
                id: REFLECTION_ITEM,
                extends: None,
                fields: vec![
                    FieldDef {
                        name: "id".into(),
                        kind: FieldKind::Primitive(Primitive::Number),
                    },
                    FieldDef {
                        name: "extendsId".into(),
                        kind: FieldKind::Primitive(Primitive::Number),
                    },
                    FieldDef {
                        name: "fields".into(),
                        kind: FieldKind::ArrayOf(FieldChild::Ref(REFLECTION_FIELD)),
                    },
                ],
            },
        );
        put(
            &mut seed.types,
            TypeDef {
                id: REFLECTION_FIELD,
                extends: None,
                fields: vec![
                    FieldDef {
                        name: "name".into(),
                        kind: FieldKind::Primitive(Primitive::String),
                    },
                    FieldDef {
                        name: "type".into(),
                        kind: FieldKind::Primitive(Primitive::String),
                    },
                    FieldDef {
                        name: "referencedType".into(),
                        kind: FieldKind::Primitive(Primitive::Number),
                    },
                    FieldDef {
                        name: "childPrimitive".into(),
                        kind: FieldKind::Primitive(Primitive::String),
                    },
                ],
            },
        );
        decode::decode_full(&mut seed, bytes)?;
        let view = decode::materialize(&seed, 0);
        let root_type = view
            .field("rootType")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as u16;
        let mut types = HashMap::new();
        let Some(items) = view.field("types").and_then(Value::items) else {
            return Err("reflection: types 배열 없음".into());
        };
        for item in items {
            let id = item.field("id").and_then(Value::as_f64).unwrap_or(-1.0) as i64;
            if id < 0 {
                continue;
            }
            let extends_raw = item
                .field("extendsId")
                .and_then(Value::as_f64)
                .unwrap_or(-1.0);
            let extends = if extends_raw >= 0.0 {
                Some(extends_raw as u16)
            } else {
                None
            };
            let mut fields = Vec::new();
            if let Some(field_items) = item.field("fields").and_then(Value::items) {
                for f in field_items {
                    let name = f
                        .field("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned();
                    let ty = f.field("type").and_then(Value::as_str).unwrap_or("");
                    let refd = f
                        .field("referencedType")
                        .and_then(Value::as_f64)
                        .unwrap_or(-1.0);
                    let child_prim = f
                        .field("childPrimitive")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let kind = match ty {
                        "ref" => FieldKind::Ref(refd as u16),
                        "map" | "array" | "set" => {
                            let child = if refd >= 0.0 {
                                FieldChild::Ref(refd as u16)
                            } else {
                                FieldChild::Primitive(Primitive::parse(child_prim).ok_or_else(
                                    || format!("reflection: 미지원 자식 원시 {child_prim}"),
                                )?)
                            };
                            if ty == "map" {
                                FieldKind::MapOf(child)
                            } else {
                                FieldKind::ArrayOf(child)
                            }
                        }
                        prim => FieldKind::Primitive(
                            Primitive::parse(prim)
                                .ok_or_else(|| format!("reflection: 미지원 타입 {prim}"))?,
                        ),
                    };
                    fields.push(FieldDef { name, kind });
                }
            }
            types.insert(
                id as u16,
                Arc::new(TypeDef {
                    id: id as u16,
                    extends,
                    fields,
                }),
            );
        }
        if !types.contains_key(&root_type) {
            return Err(format!("reflection: 루트 타입 {root_type} 미발견"));
        }
        self.types = types;
        self.root_type = root_type;
        self.arena.clear();
        self.view = Value::Nil;
        Ok(())
    }

    /// ROOM_STATE(전체) 적용 — 기존 상태를 대체한다.
    pub fn apply_full(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.types.is_empty() {
            return Err("스키마 미발견 — apply_reflection 먼저".into());
        }
        decode::decode_full(self, bytes)?;
        self.view = decode::materialize(self, 0);
        Ok(())
    }

    /// ROOM_STATE_PATCH(증분) 적용 — 상태를 갱신한다.
    pub fn apply_patch(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.types.is_empty() || !self.arena.contains_key(&0) {
            return Err("패치 전에 전체 상태 필요".into());
        }
        decode::decode_patch(self, bytes)?;
        self.view = decode::materialize(self, 0);
        Ok(())
    }

    /// 루트 상태 값 트리(읽기 전용 뷰 — apply마다 재구성).
    pub fn root(&self) -> &Value {
        &self.view
    }
}

/// Reflection 부트스트랩용 합성 타입 id — 와이어 타입 id(0,1,2,…)와 충돌 없게.
const REFLECTION_TYPE: u16 = 60000;
const REFLECTION_ITEM: u16 = 60001;
const REFLECTION_FIELD: u16 = 60002;
