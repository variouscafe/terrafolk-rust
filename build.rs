extern crate embed_resource;
use std::env;

fn main() {
    // cargo가 항상 주입하는 TARGET(빌드 스크립트 계약).
    #[allow(clippy::expect_used)]
    let target = env::var("TARGET").expect("cargo TARGET");
    if target.contains("windows") {
        // on windows we will set our game icon as icon for the executable
        embed_resource::compile("build/windows/icon.rc");
    }
}
