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
    build_deps::rerun_if_changed_paths("build.rs").unwrap();
    build_deps::rerun_if_changed_paths("webui/package.json").unwrap();
    build_deps::rerun_if_changed_paths("webui/vite.config.ts").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/**").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/assets/*").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/compinents/*").unwrap();
    build_deps::rerun_if_changed_paths("webui/src/compinents/layouts/*").unwrap();

}
