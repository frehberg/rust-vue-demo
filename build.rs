// build.rs

use npm_rs::*;

fn main() {
    let _exit_status = NpmEnv::default()
        .with_node_env(&NodeEnv::from_cargo_profile().unwrap_or_default())
        // .with_env("FOO", "bar")
        .set_path("webui")
        .init_env()
        .install(None)
        .run("build")
        .exec()
        .unwrap();
    // rebuild if build.rs is changed
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=webui/src/*");
}
