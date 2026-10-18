//! 외부 URL 열기 — 계정 OAuth(F2) 등 앱→브라우저 핸드오프. 게임 무관
//! platform 계층(safe_area JNI와 동일 레시피: ndk-context + jni 0.21).
//! 안드로이드는 ACTION_VIEW 인텐트, 그 외 플랫폼은 로그만(웹은 DOM·
//! 데스크톱은 OS별 오프너가 후속 과업 — 현재 호출부 없음).

use bevy::prelude::*;

/// URL을 시스템 브라우저로 연다. 성공 여부를 반환(호출부가 안내 토스트).
pub fn open_url(url: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        open_url_android(url)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = url;
        info!("[open_url] 미구현 플랫폼 — 요청: {url}");
        false
    }
}

/// 안드로이드 — Activity 컨텍스트에서 Intent(ACTION_VIEW, Uri.parse) 시작.
/// 실패는 false(예외 청소 후) — 앱 크래시 없이 관전 흐름 유지.
#[cfg(target_os = "android")]
fn open_url_android(url: &str) -> bool {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let vm_ptr = ctx.vm() as *mut jni::sys::JavaVM;
    let context_ptr = ctx.context() as jni::sys::jobject;
    if vm_ptr.is_null() || context_ptr.is_null() {
        return false;
    }
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(vm_ptr) }) else {
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return false;
    };
    let context = unsafe { JObject::from_raw(context_ptr) };

    // Uri.parse(url) — 인자·반환 모두 소유→차입 변환에 주의(jni 0.21 제네릭)
    let Ok(url_j) = env.new_string(url) else {
        return false;
    };
    let uri = env
        .call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&url_j)],
        )
        .and_then(|v| v.l())
        .ok();
    let Some(uri) = uri else {
        let _ = env.exception_clear();
        return false;
    };
    // Intent(Intent.ACTION_VIEW, uri) — 플래그 NEW_TASK(앱 컨텍스트 의무)
    let action = env
        .get_static_field(
            "android/content/Intent",
            "ACTION_VIEW",
            "Ljava/lang/String;",
        )
        .and_then(|v| v.l())
        .ok();
    let Some(action) = action else {
        let _ = env.exception_clear();
        return false;
    };
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )
        .ok();
    let Some(intent) = intent else {
        let _ = env.exception_clear();
        return false;
    };
    let _ = env.call_method(
        &intent,
        "setFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)],
    );
    if env
        .call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )
        .is_err()
    {
        let _ = env.exception_clear();
        info!("[open_url] startActivity 실패(브라우저 없음?)");
        return false;
    }
    info!("[open_url] 브라우저 핸드오프: {url}");
    true
}
