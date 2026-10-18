//! 딥링크 수신 — 런치 인텐트의 data URI(OAuth 콜백 terrafolk://auth#…).
//! 부팅 시 1회 판독으로 충분(토큰 수령은 1회 이벤트 — onNewIntent 재수신은
//! 후속 과업). 게임 무관 platform 계층(safe_area·open_url과 동일 레시피).

use bevy::prelude::*;

/// 런치 인텐트 data URI — 실패·비안드로이드는 None.
pub fn launch_uri() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        launch_uri_android()
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

#[cfg(target_os = "android")]
fn launch_uri_android() -> Option<String> {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let vm_ptr = ctx.vm() as *mut jni::sys::JavaVM;
    let context_ptr = ctx.context() as jni::sys::jobject;
    if vm_ptr.is_null() || context_ptr.is_null() {
        return None;
    }
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { JObject::from_raw(context_ptr) };

    let intent = env
        .call_method(&context, "getIntent", "()Landroid/content/Intent;", &[])
        .ok()?
        .l()
        .ok()?;
    let data = env
        .call_method(&intent, "getData", "()Landroid/net/Uri;", &[])
        .ok()?
        .l()
        .ok()?;
    let s = env
        .call_method(&data, "toString", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    let jstr = jni::objects::JString::from(s);
    let out = env.get_string(&jstr).ok()?.into();
    info!("[deep_link] 런치 URI: {out}");
    Some(out)
}

/// terrafolk://auth#at=..&rt=.. 조각 → (access, refresh). 순수 파서(테스트 가능).
pub fn parse_auth_fragment(uri: &str) -> Option<(String, String)> {
    let (scheme_part, frag) = uri.split_once('#')?;
    if !scheme_part.starts_with("terrafolk://") {
        return None;
    }
    let mut access = None;
    let mut refresh = None;
    for pair in frag.split('&') {
        let (k, v) = pair.split_once('=')?;
        let v = percent_decode(v);
        match k {
            "at" => access = Some(v),
            "rt" => refresh = Some(v),
            _ => {}
        }
    }
    Some((access?, refresh?))
}

/// 최소 퍼센트 디코딩(OAuth 토큰의 인코딩 문자).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            let hex = |c: u8| -> Option<u8> {
                match c {
                    b'0'..=b'9' => Some(c - b'0'),
                    b'a'..=b'f' => Some(c - b'a' + 10),
                    b'A'..=b'F' => Some(c - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_auth_fragment() {
        let (at, rt) =
            parse_auth_fragment("terrafolk://auth#at=abc%2Fx&rt=def&provider=google").unwrap();
        assert_eq!(at, "abc/x");
        assert_eq!(rt, "def");
        assert!(parse_auth_fragment("https://x/#at=1&rt=2").is_none());
        assert!(parse_auth_fragment("terrafolk://auth#at=only").is_none());
    }
}
