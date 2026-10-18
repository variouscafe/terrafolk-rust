//! msgpack — 두 방언을 다룬다(이 라이브러리의 가장 큰 함정):
//!
//! 1. **notepack 방언(LE)** — @colyseus/schema 와이어(refId·와이어인덱스·키·
//!    문자열·t.number() 값). 다중바이트 길이·float이 **리틀 엔디언**.
//!    (정본 @colyseus/schema/src/encoding/decode.ts)
//! 2. **표준 msgpack(BE)** — ROOM_DATA 페이로드(서버가 msgpackr로 인코딩)와
//!    클라→서버 송신. 다중바이트는 **빅 엔디언**.
//!
//! 두 방언 모두 실제 쓰는 서브셋만: nil/bool/정수/float/문자열/배열/맵.

/// notepack(LE) 방언 — 스키마 와이어 디코딩 전용.
pub mod notepack {
    /// msgpack 숫자 서브셋 → f64(정수 계열도 f64로 — u56 정확).
    pub fn read_number(bytes: &[u8], off: &mut usize) -> Result<f64, String> {
        let b = *bytes.get(*off).ok_or("notepack: 숫자 바이트 없음")?;
        *off += 1;
        let mut le = |n: usize| -> Result<f64, String> {
            if *off + n > bytes.len() {
                return Err("notepack: 숫자 잘림".into());
            }
            let mut buf = [0u8; 8];
            buf[..n].copy_from_slice(&bytes[*off..*off + n]);
            *off += n;
            Ok(match n {
                1 => buf[0] as f64,
                2 => u16::from_le_bytes(buf[..2].try_into().unwrap()) as f64,
                4 => u32::from_le_bytes(buf[..4].try_into().unwrap()) as f64,
                8 => u64::from_le_bytes(buf) as f64,
                _ => unreachable!(),
            })
        };
        match b {
            0x00..=0x7F => Ok(b as f64),
            0xE0..=0xFF => Ok((b as i8) as f64),
            0xCA => {
                if *off + 4 > bytes.len() {
                    return Err("notepack: f32 잘림".into());
                }
                let v = f32::from_le_bytes(bytes[*off..*off + 4].try_into().unwrap());
                *off += 4;
                Ok(v as f64)
            }
            0xCB => {
                if *off + 8 > bytes.len() {
                    return Err("notepack: f64 잘림".into());
                }
                let v = f64::from_le_bytes(bytes[*off..*off + 8].try_into().unwrap());
                *off += 8;
                Ok(v)
            }
            0xCC => le(1),
            0xCD => le(2),
            0xCE => le(4),
            0xCF => le(8),
            0xD0 => Ok(le(1)? as i8 as f64),
            0xD1 => Ok(le(2)? as i16 as f64),
            0xD2 => Ok(le(4)? as i32 as f64),
            0xD3 => {
                let v = le(8)? as u64;
                Ok(v as i64 as f64)
            }
            _ => Err(format!("notepack: 알 수 없는 숫자 타입 0x{b:02X}")),
        }
    }

    /// msgpack 문자열(LE 길이) — 스키마의 문자열 키·t.string() 값.
    pub fn read_string(bytes: &[u8], off: &mut usize) -> Result<String, String> {
        let b = *bytes.get(*off).ok_or("notepack: 문자열 헤더 없음")?;
        *off += 1;
        let len = match b {
            0xA0..=0xBF => (b & 0x1F) as usize,
            0xD9 => {
                *off += 1;
                *bytes.get(*off - 1).unwrap_or(&0) as usize
            }
            0xDA => read_le_len(bytes, off, 2)?,
            0xDB => read_le_len(bytes, off, 4)?,
            _ => return Err(format!("notepack: 문자열 아님 0x{b:02X}")),
        };
        if *off + len > bytes.len() {
            return Err("notepack: 문자열 잘림".into());
        }
        let s = String::from_utf8_lossy(&bytes[*off..*off + len]).into_owned();
        *off += len;
        Ok(s)
    }

    fn read_le_len(bytes: &[u8], off: &mut usize, n: usize) -> Result<usize, String> {
        if *off + n > bytes.len() {
            return Err("notepack: 길이 잘림".into());
        }
        let mut buf = [0u8; 4];
        buf[..n].copy_from_slice(&bytes[*off..*off + n]);
        *off += n;
        Ok(match n {
            2 => u16::from_le_bytes(buf[..2].try_into().unwrap()) as usize,
            _ => u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize,
        })
    }
}

/// 표준 msgpack(BE) 읽기 — ROOM_DATA 페이로드 → serde_json::Value.
pub fn read_value(bytes: &[u8], off: &mut usize) -> Result<serde_json::Value, String> {
    use serde_json::Value as V;
    let b = *bytes.get(*off).ok_or("msgpack: 바이트 없음")?;
    *off += 1;
    let mut be = |n: usize| -> Result<u64, String> {
        if *off + n > bytes.len() {
            return Err("msgpack: 정수 잘림".into());
        }
        let mut buf = [0u8; 8];
        buf[8 - n..].copy_from_slice(&bytes[*off..*off + n]);
        *off += n;
        Ok(u64::from_be_bytes(buf))
    };
    let val = match b {
        0xC0 => V::Null,
        0xC1 => return Err("msgpack: 예약 바이트 0xC1".into()),
        0xC2 => V::Bool(false),
        0xC3 => V::Bool(true),
        0x00..=0x7F => V::from(b),
        0xE0..=0xFF => V::from((b as i8) as i64),
        0xCA => {
            if *off + 4 > bytes.len() {
                return Err("msgpack: f32 잘림".into());
            }
            let v = f32::from_be_bytes(bytes[*off..*off + 4].try_into().unwrap());
            *off += 4;
            V::from(v as f64)
        }
        0xCB => {
            if *off + 8 > bytes.len() {
                return Err("msgpack: f64 잘림".into());
            }
            let v = f64::from_be_bytes(bytes[*off..*off + 8].try_into().unwrap());
            *off += 8;
            serde_json::Number::from_f64(v)
                .map(V::Number)
                .unwrap_or(V::Null)
        }
        0xCC => V::from(be(1)? as u8),
        0xCD => V::from(be(2)? as u16),
        0xCE => V::from(be(4)? as u32),
        0xCF => V::from(be(8)?),
        0xD0 => V::from(be(1)? as u8 as i8 as i64),
        0xD1 => V::from(be(2)? as u16 as i16 as i64),
        0xD2 => V::from(be(4)? as u32 as i32 as i64),
        0xD3 => V::from(be(8)? as i64),
        0xA0..=0xBF | 0xD9 | 0xDA | 0xDB => {
            *off -= 1;
            V::String(read_string(bytes, off)?)
        }
        0xC4..=0xC6 => {
            // bin — 이 방언에선 미사용(길이만 건너뛴다)
            let len = match b {
                0xC4 => {
                    let l = *bytes.get(*off).ok_or("msgpack: bin8 잘림")? as usize;
                    *off += 1;
                    l
                }
                0xC5 => {
                    let l = u16::from_be_bytes(
                        bytes
                            .get(*off..*off + 2)
                            .ok_or("msgpack: bin16 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 2;
                    l
                }
                _ => {
                    let l = u32::from_be_bytes(
                        bytes
                            .get(*off..*off + 4)
                            .ok_or("msgpack: bin32 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 4;
                    l
                }
            };
            *off += len;
            V::Null
        }
        0x90..=0x9F | 0xDC | 0xDD => {
            let len = match b {
                0x90..=0x9F => (b & 0x0F) as usize,
                0xDC => {
                    let l = u16::from_be_bytes(
                        bytes
                            .get(*off..*off + 2)
                            .ok_or("msgpack: 배열 길이 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 2;
                    l
                }
                _ => {
                    let l = u32::from_be_bytes(
                        bytes
                            .get(*off..*off + 4)
                            .ok_or("msgpack: 배열 길이 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 4;
                    l
                }
            };
            let mut arr = Vec::with_capacity(len.min(64));
            for _ in 0..len {
                arr.push(read_value(bytes, off)?);
            }
            V::Array(arr)
        }
        0x80..=0x8F | 0xDE | 0xDF => {
            let len = match b {
                0x80..=0x8F => (b & 0x0F) as usize,
                0xDE => {
                    let l = u16::from_be_bytes(
                        bytes
                            .get(*off..*off + 2)
                            .ok_or("msgpack: 맵 길이 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 2;
                    l
                }
                _ => {
                    let l = u32::from_be_bytes(
                        bytes
                            .get(*off..*off + 4)
                            .ok_or("msgpack: 맵 길이 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 4;
                    l
                }
            };
            let mut map = serde_json::Map::with_capacity(len.min(64));
            for _ in 0..len {
                let k = match read_value(bytes, off)? {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                let v = read_value(bytes, off)?;
                map.insert(k, v);
            }
            V::Object(map)
        }
        0xC7 | 0xC8 | 0xC9 | 0xD4..=0xD8 => {
            // ext/fixext — 이 방언 미사용, 길이 계산 후 건너뛴다
            let len = match b {
                0xD4..=0xD8 => (1 << (b as usize - 0xD4)).min(16),
                0xC7 => {
                    let l = *bytes.get(*off).ok_or("msgpack: ext8 잘림")? as usize;
                    *off += 1;
                    l
                }
                0xC8 => {
                    let l = u16::from_be_bytes(
                        bytes
                            .get(*off..*off + 2)
                            .ok_or("msgpack: ext16 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 2;
                    l
                }
                _ => {
                    let l = u32::from_be_bytes(
                        bytes
                            .get(*off..*off + 4)
                            .ok_or("msgpack: ext32 잘림")?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    *off += 4;
                    l
                }
            };
            *off += len + 1; // 본문 + 타입 바이트
            serde_json::Value::Null
        }
    };
    Ok(val)
}

/// 표준 msgpack(BE) 문자열 — ROOM_DATA 타입 이름 추출용.
pub fn read_string(bytes: &[u8], off: &mut usize) -> Result<String, String> {
    let b = *bytes.get(*off).ok_or("msgpack: 문자열 헤더 없음")?;
    *off += 1;
    let len = match b {
        0xA0..=0xBF => (b & 0x1F) as usize,
        0xD9 => {
            let l = *bytes.get(*off).ok_or("msgpack: str8 잘림")? as usize;
            *off += 1;
            l
        }
        0xDA => {
            if *off + 2 > bytes.len() {
                return Err("msgpack: str16 잘림".into());
            }
            let l = u16::from_be_bytes(bytes[*off..*off + 2].try_into().unwrap()) as usize;
            *off += 2;
            l
        }
        0xDB => {
            if *off + 4 > bytes.len() {
                return Err("msgpack: str32 잘림".into());
            }
            let l = u32::from_be_bytes(bytes[*off..*off + 4].try_into().unwrap()) as usize;
            *off += 4;
            l
        }
        _ => return Err(format!("msgpack: 문자열 아님 0x{b:02X}")),
    };
    if *off + len > bytes.len() {
        return Err("msgpack: 문자열 잘림".into());
    }
    let s = String::from_utf8_lossy(&bytes[*off..*off + len]).into_owned();
    *off += len;
    Ok(s)
}

/// 표준 msgpack(BE) 쓰기 — 송신 페이로드 직렬화.
pub fn write_string(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    match b.len() {
        0..=31 => out.push(0xA0 | b.len() as u8),
        32..=255 => {
            out.push(0xD9);
            out.push(b.len() as u8);
        }
        256..=65535 => {
            out.push(0xDA);
            out.extend_from_slice(&(b.len() as u16).to_be_bytes());
        }
        _ => {
            out.push(0xDB);
            out.extend_from_slice(&(b.len() as u32).to_be_bytes());
        }
    }
    out.extend_from_slice(b);
}

pub fn write_value(out: &mut Vec<u8>, v: &serde_json::Value) {
    use serde_json::Value as V;
    match v {
        V::Null => out.push(0xC0),
        V::Bool(b) => out.push(if *b { 0xC3 } else { 0xC2 }),
        V::Number(n) => {
            if let Some(i) = n.as_i64() {
                write_int(out, i);
            } else if let Some(u) = n.as_u64() {
                write_uint(out, u);
            } else if let Some(f) = n.as_f64() {
                out.push(0xCB);
                out.extend_from_slice(&f.to_be_bytes());
            }
        }
        V::String(s) => write_string(out, s),
        V::Array(arr) => match arr.len() {
            0..=15 => out.push(0x90 | arr.len() as u8),
            16..=65535 => {
                out.push(0xDC);
                out.extend_from_slice(&(arr.len() as u16).to_be_bytes());
            }
            _ => {
                out.push(0xDD);
                out.extend_from_slice(&(arr.len() as u32).to_be_bytes());
            }
        },
        V::Object(map) => match map.len() {
            0..=15 => out.push(0x80 | map.len() as u8),
            16..=65535 => {
                out.push(0xDE);
                out.extend_from_slice(&(map.len() as u16).to_be_bytes());
            }
            _ => {
                out.push(0xDF);
                out.extend_from_slice(&(map.len() as u32).to_be_bytes());
            }
        },
    }
    match v {
        V::Array(arr) => {
            for item in arr {
                write_value(out, item);
            }
        }
        V::Object(map) => {
            for (k, val) in map {
                write_string(out, k);
                write_value(out, val);
            }
        }
        _ => {}
    }
}

fn write_int(out: &mut Vec<u8>, i: i64) {
    match i {
        -32..=127 => out.push(i as i8 as u8),
        -128..=-33 => {
            out.push(0xD0);
            out.push(i as i8 as u8);
        }
        -32768..=-129 => {
            out.push(0xD1);
            out.extend_from_slice(&(i as i16).to_be_bytes());
        }
        -2147483648..=-32769 => {
            out.push(0xD2);
            out.extend_from_slice(&(i as i32).to_be_bytes());
        }
        _ => {
            out.push(0xD3);
            out.extend_from_slice(&i.to_be_bytes());
        }
    }
}

fn write_uint(out: &mut Vec<u8>, u: u64) {
    match u {
        0..=127 => out.push(u as u8),
        128..=255 => {
            out.push(0xCC);
            out.push(u as u8);
        }
        256..=65535 => {
            out.push(0xCD);
            out.extend_from_slice(&(u as u16).to_be_bytes());
        }
        65536..=4294967295 => {
            out.push(0xCE);
            out.extend_from_slice(&(u as u32).to_be_bytes());
        }
        _ => {
            out.push(0xCF);
            out.extend_from_slice(&u.to_be_bytes());
        }
    }
}
