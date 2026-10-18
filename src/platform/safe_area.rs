//! 시스템 UI 세이프에어리어 — 상태바·내비게이션 바에 UI가 깔리지 않게.
//! 게임 무관 재사용: `SafeArea` 리소스를 읽어 UI 배치에 더한다.
//! 안드로이드는 JNI로 WindowInsets를 물어보고(부팅 1회), 나머지 플랫폼은 0.
//!
//! ndk-context의 context()는 글루에 따라 Activity(game-activity) 또는
//! Application(native-activity — 이 앱)이다. Activity면 getWindow 경로,
//! Application이면 WindowManager.getCurrentWindowMetrics(API 30+) 경로로
//! 인셋을 얻는다. JNI 실패는 예외를 반드시 청소하고 (0, 0)으로 낙관
//! (미청소 예외는 다음 JNI 호출을 abort시킨다 — 2026-10-18 실측 크래시).

use bevy::prelude::*;

/// 세이프에어리어 인셋(논리 px) — top은 상태바/노치, bottom은 내비 바.
/// 실패 시 0(풀스크린 가정). 회전 전환은 미지원 — 필요 시 리프레시 확장.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SafeArea {
    pub top: f32,
    pub bottom: f32,
}

pub fn setup(app: &mut App) {
    let area = SafeArea {
        top: query_top(),
        bottom: query_bottom(),
    };
    info!("[platform] safe area(빌드 시 1차) {:?}", area);
    app.insert_resource(area);
}

/// 인셋 재쿼리 — 빌드 시점엔 첫 레이아웃 전이라 0이 올 수 있다. Startup과
/// 부팅 0.5초 뒤(UI 스폰 전후)에 다시 물어본다. 값이 변하면 로그.
pub fn refresh(time: Option<Res<Time>>, mut safe: ResMut<SafeArea>, mut late: Local<bool>) {
    let due_now = time.is_none(); // Startup 호출
    let due_late = match &time {
        Some(t) => !*late && t.elapsed_secs() > 0.5,
        None => false,
    };
    if !due_now && !due_late {
        return;
    }
    if time.is_some() {
        *late = true;
    }
    let fresh = SafeArea {
        top: query_top(),
        bottom: query_bottom(),
    };
    if (fresh.top - safe.top).abs() > 0.5 || (fresh.bottom - safe.bottom).abs() > 0.5 {
        info!("[platform] safe area 갱신 {:?} → {:?}", safe, fresh);
        *safe = fresh;
    }
}

#[cfg(target_os = "android")]
fn query_top() -> f32 {
    let (top, _) = query_android_insets();
    top
}

#[cfg(target_os = "android")]
fn query_bottom() -> f32 {
    let (_, bottom) = query_android_insets();
    bottom
}

#[cfg(not(target_os = "android"))]
fn query_top() -> f32 {
    0.0
}

#[cfg(not(target_os = "android"))]
fn query_bottom() -> f32 {
    0.0
}

/// 안드로이드 시스템 인셋(top, bottom) px. 실패는 (0, 0)으로 침묵 낙관.
#[cfg(target_os = "android")]
fn query_android_insets() -> (f32, f32) {
    use jni::objects::JObject;

    let ctx = ndk_context::android_context();
    let vm_ptr = ctx.vm() as *mut jni::sys::JavaVM;
    let context_ptr = ctx.context() as jni::sys::jobject;
    if vm_ptr.is_null() || context_ptr.is_null() {
        return (0.0, 0.0);
    }
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(vm_ptr) }) else {
        return (0.0, 0.0);
    };
    // 메인 스레드는 보통 이미 attach — 가드는 이미 attach된 스레드를 해제하지
    // 않는다(jni 0.21 계약). 부팅 1회 호출이라 비용도 무의미.
    let Ok(mut env) = vm.attach_current_thread() else {
        return (0.0, 0.0);
    };
    let context = unsafe { JObject::from_raw(context_ptr) };

    // 실패 시 대기 중인 Java 예외 청소 + None
    macro_rules! obj {
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

    // ── WindowInsets 확보 — 컨텍스트 종류에 따라 두 경로
    let is_activity = env
        .find_class("android/app/Activity")
        .ok()
        .and_then(|c| env.is_instance_of(&context, &c).ok())
        .unwrap_or(false);
    let insets: Option<JObject> = (|| {
        if is_activity {
            // Activity: getWindow().getDecorView().getRootWindowInsets()
            let window = obj!(&context, "getWindow", "()Landroid/view/Window;")?;
            let view = obj!(&window, "getDecorView", "()Landroid/view/View;")?;
            obj!(
                &view,
                "getRootWindowInsets",
                "()Landroid/view/WindowInsets;"
            )
        } else {
            // Application: getSystemService("window") → getCurrentWindowMetrics
            // (API 30+) → getWindowInsets()
            let service = env.new_string("window").ok().map(JObject::from)?;
            let wm = obj!(
                &context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                jni::objects::JValue::Object(&service),
            )?;
            let metrics = obj!(
                &wm,
                "getCurrentWindowMetrics",
                "()Landroid/view/WindowMetrics;",
            )?;
            obj!(&metrics, "getWindowInsets", "()Landroid/view/WindowInsets;",)
        }
    })();
    let Some(insets) = insets else {
        let _ = env.exception_clear();
        return (0.0, 0.0);
    };
    let mut inset = |name: &str| -> f32 {
        match env.call_method(&insets, name, "()I", &[]) {
            Ok(v) => v.i().unwrap_or(0) as f32,
            Err(_) => {
                let _ = env.exception_clear();
                0.0
            }
        }
    };
    (
        inset("getSystemWindowInsetTop"),
        inset("getSystemWindowInsetBottom"),
    )
}
