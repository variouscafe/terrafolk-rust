//! 스키마 와이어 디코더 — @colyseus/schema 5.x 오퍼레이션 스트림.
//!
//! 문법(정본 src/decoder/DecodeOperation.ts·Decoder.ts):
//! - `0xFF + msgpack(refId)` = 구조체 스위치. 프레임 시작의 현재 ref = 루트(0).
//! - 구조체 옵바이트: 상위 2비트 연산(REPLACE 00 / DELETE 01 / ADD 10 /
//!   DELETE_AND_ADD 11) + 하위 6비트 필드 인덱스(0..62).
//! - 컬렉션(Map/Array) 옵바이트: 연산값 그대로(128/64/192/10/15/…) 뒤에
//!   msgpack 와이어인덱스, (ADD 계열이면) msgpack 문자열 키, 자식 값.
//! - 자식 구조체·컬렉션 값 = msgpack refId — 실체는 이후 스위치로 도착.
//!   (ADD 비트가 있을 때 `213 + msgpack(타입id)` 접두가 선택적으로 붙는다.)

use super::{FieldChild, FieldKind, Primitive, RefNode, SchemaState, TypeDef, Value};
use crate::msgpack::notepack;
use std::collections::HashMap;
use std::sync::Arc;

const SWITCH: u8 = 0xFF;
const TYPE_ID_MARK: u8 = 213;
const OP_DELETE: u8 = 64;
const OP_ADD: u8 = 128;
const OP_DELETE_AND_ADD: u8 = 192;
const OP_CLEAR: u8 = 10;
const OP_REVERSE: u8 = 15;
const OP_MOVE: u8 = 32;
const OP_DELETE_AND_MOVE: u8 = 96;
const OP_MOVE_AND_ADD: u8 = 160;
const OP_DELETE_BY_REFID: u8 = 33;
const OP_ADD_BY_REFID: u8 = 129;

/// 전체 상태(ROOM_STATE) — 아레나를 초기화하고 루트(0)부터 디코딩.
pub(crate) fn decode_full(state: &mut SchemaState, bytes: &[u8]) -> Result<(), String> {
    state.arena.clear();
    let root_def = state
        .types
        .get(&state.root_type)
        .ok_or_else(|| format!("루트 타입 {} 정의 없음", state.root_type))?
        .clone();
    state.arena.insert(
        0,
        RefNode::Struct {
            def: root_def,
            fields: Vec::new(),
        },
    );
    decode_stream(state, bytes, 0)
}

/// 증분 패치 — 기존 아레나에 적용.
pub(crate) fn decode_patch(state: &mut SchemaState, bytes: &[u8]) -> Result<(), String> {
    decode_stream(state, bytes, 0)
}

fn decode_stream(state: &mut SchemaState, bytes: &[u8], root: u32) -> Result<(), String> {
    let mut off = 0usize;
    let mut current = root;
    while off < bytes.len() {
        let b = bytes[off];
        if b == SWITCH {
            off += 1;
            let ref_id = notepack::read_number(bytes, &mut off)? as u32;
            if !state.arena.contains_key(&ref_id) {
                return Err(format!("스위치: 미등록 ref {ref_id} — 스트림 불일치"));
            }
            current = ref_id;
            continue;
        }
        // 현재 노드 종류별 분기 — borrows 정리를 위해 필요한 것만 미리 복제.
        enum Cur {
            Struct(Arc<TypeDef>),
            Map(FieldChild),
            Array(FieldChild),
        }
        let cur = {
            let node = state
                .arena
                .get(&current)
                .ok_or_else(|| format!("현재 ref {current} 없음"))?;
            match node {
                RefNode::Struct { def, .. } => Cur::Struct(def.clone()),
                RefNode::Map { child, .. } => Cur::Map(*child),
                RefNode::Array { child, .. } => Cur::Array(*child),
            }
        };
        match cur {
            Cur::Struct(def) => {
                let op = b & 0xC0;
                let idx = (b & 0x3F) as usize;
                off += 1;
                let field = def
                    .fields
                    .get(idx)
                    .ok_or_else(|| format!("타입 {} 필드 {idx} 미지정 — 스키마 불일치", def.id))?;
                if op == OP_DELETE {
                    if let Some(RefNode::Struct { fields, .. }) = state.arena.get_mut(&current)
                        && idx < fields.len()
                    {
                        fields[idx] = None;
                    }
                    continue;
                }
                let value =
                    read_field_value(state, &field.kind, op & OP_ADD != 0, bytes, &mut off)?;
                if let Some(RefNode::Struct { fields, .. }) = state.arena.get_mut(&current) {
                    if fields.len() <= idx {
                        fields.resize(idx + 1, None);
                    }
                    fields[idx] = Some(value);
                }
            }
            Cur::Map(child) => {
                off += 1;
                if b == OP_CLEAR {
                    if let Some(RefNode::Map {
                        index_key, entries, ..
                    }) = state.arena.get_mut(&current)
                    {
                        index_key.clear();
                        entries.clear();
                    }
                    continue;
                }
                let wire = notepack::read_number(bytes, &mut off)? as u64;
                if b == OP_DELETE {
                    let key = state.arena.get(&current).and_then(|n| match n {
                        RefNode::Map { index_key, .. } => index_key
                            .iter()
                            .find(|(w, _)| *w == wire)
                            .map(|(_, k)| k.clone()),
                        _ => None,
                    });
                    if let Some(key) = key
                        && let Some(RefNode::Map {
                            index_key, entries, ..
                        }) = state.arena.get_mut(&current)
                    {
                        index_key.retain(|(w, _)| *w != wire);
                        entries.retain(|(k, _)| *k != key);
                    }
                    continue;
                }
                let key = if b & OP_ADD != 0 {
                    notepack::read_string(bytes, &mut off)?
                } else {
                    state
                        .arena
                        .get(&current)
                        .and_then(|n| match n {
                            RefNode::Map { index_key, .. } => index_key
                                .iter()
                                .find(|(w, _)| *w == wire)
                                .map(|(_, k)| k.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("~{wire}"))
                };
                // DELETE_AND_ADD — 같은 와이어 인덱스의 기존 엔트리를 먼저 지운다.
                if b == OP_DELETE_AND_ADD {
                    let old_key = state.arena.get(&current).and_then(|n| match n {
                        RefNode::Map { index_key, .. } => index_key
                            .iter()
                            .find(|(w, _)| *w == wire)
                            .map(|(_, k)| k.clone()),
                        _ => None,
                    });
                    if let Some(old) = old_key
                        && let Some(RefNode::Map {
                            index_key, entries, ..
                        }) = state.arena.get_mut(&current)
                    {
                        index_key.retain(|(w, _)| *w != wire);
                        entries.retain(|(k, _)| *k != old);
                    }
                }
                let value = read_child_value(state, child, bytes, &mut off)?;
                if let Some(RefNode::Map {
                    index_key, entries, ..
                }) = state.arena.get_mut(&current)
                {
                    index_key.retain(|(w, _)| *w != wire);
                    index_key.push((wire, key.clone()));
                    if let Some(slot) = entries.iter_mut().find(|(k, _)| *k == key) {
                        slot.1 = value;
                    } else {
                        entries.push((key, value));
                    }
                }
            }
            Cur::Array(child) => {
                off += 1;
                match b {
                    OP_CLEAR => {
                        if let Some(RefNode::Array { items, .. }) = state.arena.get_mut(&current) {
                            items.clear();
                        }
                    }
                    OP_REVERSE => {
                        if let Some(RefNode::Array { items, .. }) = state.arena.get_mut(&current) {
                            items.reverse();
                        }
                    }
                    OP_ADD | 0 => {
                        let idx = notepack::read_number(bytes, &mut off)? as usize;
                        let value = read_child_value(state, child, bytes, &mut off)?;
                        if let Some(RefNode::Array { items, .. }) = state.arena.get_mut(&current) {
                            if idx < items.len() {
                                items[idx] = value;
                            } else {
                                while items.len() < idx {
                                    items.push(Value::Nil);
                                }
                                items.push(value);
                            }
                        }
                    }
                    OP_DELETE => {
                        let idx = notepack::read_number(bytes, &mut off)? as usize;
                        if let Some(RefNode::Array { items, .. }) = state.arena.get_mut(&current)
                            && idx < items.len()
                        {
                            items.remove(idx);
                        }
                    }
                    OP_MOVE | OP_DELETE_AND_MOVE | OP_MOVE_AND_ADD => {
                        // 배열 이동 계열 — 본 서버(reflection 포함)는 ADD만
                        // 쓴다. 방어적 소비: [from][to](+MOVE_AND_ADD는 값 1개).
                        let _from = notepack::read_number(bytes, &mut off)?;
                        let _to = notepack::read_number(bytes, &mut off)?;
                        if b == OP_MOVE_AND_ADD {
                            let _ = read_child_value(state, child, bytes, &mut off)?;
                        }
                    }
                    OP_DELETE_BY_REFID | OP_ADD_BY_REFID => {
                        let _ref = notepack::read_number(bytes, &mut off)?;
                        if b == OP_ADD_BY_REFID {
                            let _ = read_child_value(state, child, bytes, &mut off)?;
                        }
                    }
                    other => return Err(format!("배열 미지원 연산 {other}")),
                }
            }
        }
    }
    Ok(())
}

/// 구조체 필드 값 — 원시는 즉시, 참조·컬렉션은 refId 노드 등록 후 자리표.
fn read_field_value(
    state: &mut SchemaState,
    kind: &FieldKind,
    has_add: bool,
    bytes: &[u8],
    off: &mut usize,
) -> Result<Value, String> {
    match kind {
        FieldKind::Primitive(p) => read_primitive(*p, bytes, off),
        FieldKind::Ref(declared) => {
            let ref_id = notepack::read_number(bytes, off)? as u32;
            let mut type_id = *declared;
            if has_add && bytes.get(*off) == Some(&TYPE_ID_MARK) {
                *off += 1;
                type_id = notepack::read_number(bytes, off)? as u16;
            }
            register_struct(state, ref_id, type_id)?;
            Ok(Value::Ref(ref_id))
        }
        FieldKind::MapOf(child) | FieldKind::ArrayOf(child) => {
            let ref_id = notepack::read_number(bytes, off)? as u32;
            if has_add && bytes.get(*off) == Some(&TYPE_ID_MARK) {
                *off += 1;
                let _ = notepack::read_number(bytes, off)?; // 컬렉션 타입id(무시)
            }
            state.arena.entry(ref_id).or_insert_with(|| match kind {
                FieldKind::MapOf(_) => RefNode::Map {
                    child: *child,
                    index_key: Vec::new(),
                    entries: Vec::new(),
                },
                _ => RefNode::Array {
                    child: *child,
                    items: Vec::new(),
                },
            });
            Ok(Value::Ref(ref_id))
        }
    }
}

/// 컬렉션 엔트리 값.
fn read_child_value(
    state: &mut SchemaState,
    child: FieldChild,
    bytes: &[u8],
    off: &mut usize,
) -> Result<Value, String> {
    match child {
        FieldChild::Primitive(p) => read_primitive(p, bytes, off),
        FieldChild::Ref(declared) => {
            let ref_id = notepack::read_number(bytes, off)? as u32;
            // 엔트리 추가에도 선택적 타입id 접두가 올 수 있다(선언과 다를 때).
            if bytes.get(*off) == Some(&TYPE_ID_MARK) {
                *off += 1;
                let type_id = notepack::read_number(bytes, off)? as u16;
                register_struct(state, ref_id, type_id)?;
            } else {
                register_struct(state, ref_id, declared)?;
            }
            Ok(Value::Ref(ref_id))
        }
    }
}

fn register_struct(state: &mut SchemaState, ref_id: u32, type_id: u16) -> Result<(), String> {
    if state.arena.contains_key(&ref_id) {
        return Ok(()); // 기존 인스턴스 재참조
    }
    let def = state
        .types
        .get(&type_id)
        .ok_or_else(|| format!("타입 {type_id} 정의 없음(ref {ref_id})"))?
        .clone();
    state.arena.insert(
        ref_id,
        RefNode::Struct {
            def,
            fields: Vec::new(),
        },
    );
    Ok(())
}

/// 원시값 — 타입별 인코딩 3갈래(Number/String=notepack, Bool=원시1B, 나머지 원시 LE).
fn read_primitive(p: Primitive, bytes: &[u8], off: &mut usize) -> Result<Value, String> {
    let le = |n: usize, off: &mut usize| -> Result<f64, String> {
        if *off + n > bytes.len() {
            return Err("원시 숫자 잘림".into());
        }
        let mut buf = [0u8; 8];
        buf[..n].copy_from_slice(&bytes[*off..*off + n]);
        *off += n;
        Ok(match n {
            1 => buf[0] as f64,
            2 => u16::from_le_bytes(buf[..2].try_into().unwrap()) as f64,
            4 => u32::from_le_bytes(buf[..4].try_into().unwrap()) as f64,
            _ => u64::from_le_bytes(buf) as f64,
        })
    };
    Ok(match p {
        Primitive::Bool => {
            let b = *bytes.get(*off).ok_or("bool 잘림")?;
            *off += 1;
            Value::Bool(b != 0)
        }
        Primitive::Number => Value::Num(notepack::read_number(bytes, off)?),
        Primitive::String => Value::Str(notepack::read_string(bytes, off)?),
        Primitive::I8 => Value::Num(le(1, off)? as i8 as f64),
        Primitive::U8 => Value::Num(le(1, off)?),
        Primitive::I16 => Value::Num(le(2, off)? as u16 as i16 as f64),
        Primitive::U16 => Value::Num(le(2, off)?),
        Primitive::I32 => Value::Num(le(4, off)? as u32 as i32 as f64),
        Primitive::U32 => Value::Num(le(4, off)?),
        // i64/u64는 f64로 — 2^53 넘는 값의 정밀도 손실은 감수(문서화됨).
        Primitive::I64 => Value::Num(le(8, off)? as i64 as f64),
        Primitive::U64 => Value::Num(le(8, off)?),
        Primitive::F32 => {
            if *off + 4 > bytes.len() {
                return Err("f32 잘림".into());
            }
            let v = f32::from_le_bytes(bytes[*off..*off + 4].try_into().unwrap());
            *off += 4;
            Value::Num(v as f64)
        }
        Primitive::F64 => {
            if *off + 8 > bytes.len() {
                return Err("f64 잘림".into());
            }
            let v = f64::from_le_bytes(bytes[*off..*off + 8].try_into().unwrap());
            *off += 8;
            Value::Num(v)
        }
    })
}

/// 아레나 → 값 트리 재구성. 자리표 `Ref`를 실제 노드로 푼다(루트 refId 0).
pub(crate) fn materialize(state: &SchemaState, root: u32) -> Value {
    resolve_node(state, root, 0)
}

fn resolve_node(state: &SchemaState, ref_id: u32, depth: u32) -> Value {
    if depth > 64 {
        return Value::Nil; // 순환 방어(스키마는 비순환 — 보험)
    }
    match state.arena.get(&ref_id) {
        Some(RefNode::Struct { def, fields }) => Value::Map(
            def.fields
                .iter()
                .enumerate()
                .filter_map(|(i, f)| {
                    fields
                        .get(i)
                        .and_then(|v| v.clone())
                        .map(|v| (f.name.clone(), resolve_value(state, v, depth)))
                })
                .collect(),
        ),
        Some(RefNode::Map { entries, .. }) => Value::Map(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), resolve_value(state, v.clone(), depth)))
                .collect(),
        ),
        Some(RefNode::Array { items, .. }) => Value::Arr(
            items
                .iter()
                .map(|v| resolve_value(state, v.clone(), depth))
                .collect(),
        ),
        None => Value::Nil,
    }
}

fn resolve_value(state: &SchemaState, v: Value, depth: u32) -> Value {
    match v {
        Value::Ref(id) => resolve_node(state, id, depth + 1),
        other => other,
    }
}

/// 테스트·부트스트랩 지원 — 타입 정의 직접 주입(reflection 없이).
pub fn install_types(state: &mut SchemaState, root_type: u16, defs: Vec<TypeDef>) {
    state.types = HashMap::new();
    for def in defs {
        state.types.insert(def.id, Arc::new(def));
    }
    state.root_type = root_type;
    state.arena.clear();
}
