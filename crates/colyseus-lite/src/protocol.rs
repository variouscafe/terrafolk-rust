//! Colyseus 0.18 프레임 프로토콜 — 코드 상수·프레임 빌더·JOIN_ROOM 파서.
//! 정본: @colyseus/core 0.18 `Protocol.ts`(room/node_modules — 서버 의존과
//! 바이트 단위로 일치. 코드는 `byte & 0x1F`, 수정자 비트는 `byte & 0xE0`).

/// 프레임 코드(첫 바이트 하위 5비트).
pub mod code {
    pub const JOIN_ROOM: u8 = 10;
    pub const ERROR: u8 = 11;
    pub const LEAVE_ROOM: u8 = 12;
    pub const ROOM_DATA: u8 = 13;
    pub const ROOM_STATE: u8 = 14;
    pub const ROOM_STATE_PATCH: u8 = 15;
    pub const ROOM_DATA_BYTES: u8 = 17;
    pub const PING: u8 = 18;
}

/// 첫 바이트에서 프로토콜 코드 추출(수정자 비트 마스킹).
pub fn frame_code(first: u8) -> u8 {
    first & 0x1F
}

/// JOIN_ROOM(10) 파싱 결과.
#[derive(Debug, Clone)]
pub struct JoinInfo {
    pub reconnection_token: String,
    pub serializer_id: String,
    /// Reflection 바이트 — `SchemaState::apply_reflection`으로.
    pub reflection: Vec<u8>,
}

/// 서버 → 클라 조인 프레임 파서.
/// 레이아웃(코어 Protocol.ts 주석 정본):
/// `[10][rt-len u8][rt][sid-len u8][sid][handshakeLen msgpack-num][handshake…]`
/// `body`는 첫 바이트(코드)를 제외한 나머지.
pub fn parse_join_room(body: &[u8]) -> Result<JoinInfo, String> {
    let mut off = 0usize;
    let rt_len = *body.first().ok_or("JOIN_ROOM: rt 길이 없음")? as usize;
    off += 1;
    if off + rt_len > body.len() {
        return Err("JOIN_ROOM: rt 잘림".into());
    }
    let reconnection_token = String::from_utf8_lossy(&body[off..off + rt_len]).into_owned();
    off += rt_len;
    let sid_len = *body.get(off).ok_or("JOIN_ROOM: sid 길이 없음")? as usize;
    off += 1;
    if off + sid_len > body.len() {
        return Err("JOIN_ROOM: sid 잘림".into());
    }
    let serializer_id = String::from_utf8_lossy(&body[off..off + sid_len]).into_owned();
    off += sid_len;
    // handshakeLen — msgpack 숫자(notepack LE 방언)
    let hlen = crate::msgpack::notepack::read_number(body, &mut off)? as usize;
    if off + hlen > body.len() {
        return Err("JOIN_ROOM: reflection 잘림".into());
    }
    Ok(JoinInfo {
        reconnection_token,
        serializer_id,
        reflection: body[off..off + hlen].to_vec(),
    })
}

/// 조인 ACK(1바이트) — **필수**: 서버는 ACK 후에야 ROOM_STATE를 보낸다.
pub fn ack_join() -> Vec<u8> {
    vec![code::JOIN_ROOM]
}

pub fn leave_frame() -> Vec<u8> {
    vec![code::LEAVE_ROOM]
}

pub fn ping_frame() -> Vec<u8> {
    vec![code::PING]
}

/// ROOM_DATA(13) 송신 프레임 — `[13][msgpack 문자열 타입][msgpack 페이로드]`.
/// 페이로드는 표준 msgpack(BE)로 직렬화(서버가 msgpackr로 읽는다).
pub fn data_frame(kind: &str, payload: &serde_json::Value) -> Vec<u8> {
    let mut out = vec![code::ROOM_DATA];
    crate::msgpack::write_string(&mut out, kind);
    crate::msgpack::write_value(&mut out, payload);
    out
}
