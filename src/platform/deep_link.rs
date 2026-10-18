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
    // ⚠️ ndk-context의 context는 Application(android-activity 0.6 계약)이라
    // getIntent가 없다(2026-10-19 실측 NoSuchMethodError 즉사 사고). 공개 API인
    // ActivityManager.getAppTasks()[0].taskInfo.baseIntent에서 태스크 시작
    // 인텐트(= 딥링크 데이터)를 읽는다. 모든 실패는 예외 청소 후 None.
    macro_rules! call {
        ($recv:expr, $name:expr, $sig:expr $(, $arg:expr)* $(,)?) => {
            match env.call_method($recv, $name, $sig, &[$($arg),*]) {
                Ok(v) => v.l().ok(),
                Err(_) => {
                    let _ = env.exception_clear();
                    None
                }
            }
        };
    }
    let service = env.new_string("activity").ok()?;
    let am = call!(&context, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", JValue::Object(&service))?;
    let Some(tasks) = call!(&am, "getAppTasks", "()Ljava/util/List;") else {
        info!("[deep_link] getAppTasks 실패");
        return None;
    };
    let size = env
        .call_method(&tasks, "size", "()I", &[])
        .ok()
        .and_then(|v| v.i().ok())
        .unwrap_or(-1);
    info!("[deep_link] 태스크 수: {size}");
    // 태스크 여럿(런처 시작분 + 딥링크 VIEW분)일 수 있다 — 데이터가 있는
    // 첫 인텐트를 찾는다.
    for i in 0..size.max(0) {
        let idx = JValue::Int(i);
        let Some(task) = call!(&tasks, "get", "(I)Ljava/lang/Object;", idx) else {
            info!("[deep_link] 태스크[{i}] get 실패");
            continue;
        };
        let Some(info) = call!(&task, "getTaskInfo", "()Landroid/app/ActivityManager$RecentTaskInfo;") else {
            info!("[deep_link] 태스크[{i}] getTaskInfo 실패");
            continue;
        };
        // baseIntent는 TaskInfo의 공개 필드(메서드 아님) — get_field 경로.
        let intent = match env.get_field(&info, "baseIntent", "Landroid/content/Intent;") {
            Ok(v) => v.l().ok(),
            Err(_) => {
                let _ = env.exception_clear();
                None
            }
        };
        let Some(intent) = intent else {
            info!("[deep_link] 태스크[{i}] baseIntent 필드 실패");
            continue;
        };
        // 일반 런치(데이터 없음)는 조용히 스킵 — 부팅마다 발생.
        let Some(data) = call!(&intent, "getData", "()Landroid/net/Uri;") else {
            continue;
        };
        let Some(s) = call!(&data, "toString", "()Ljava/lang/String;") else {
            continue;
        };
        let js = jni::objects::JString::from(s);
        let Ok(jstr) = env.get_string(&js) else {
            continue;
        };
        let out: String = jstr.to_string_lossy().into_owned();
        info!("[deep_link] 태스크[{i}] URI: {out}");
        return Some(out);
    }
    None
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
