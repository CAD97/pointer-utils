use std::env;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_extern_type, has_never, enforce_1_1_0_semantics)");

    let cfg = autocfg::new();

    cfg.emit_expression_cfg("{ extern { type T; } () }", "has_extern_type");
    cfg.emit_type_cfg("!", "has_never");

    if let Ok(var) = env::var("ERASABLE_ENFORCE_1_1_0_SEMANTICS") {
        if !var.is_empty() && var != "0" {
            autocfg::emit("enforce_1_1_0_semantics");
        }
    }

    autocfg::rerun_env("ERASABLE_ENFORCE_1_1_0_SEMANTICS");
    autocfg::rerun_path("build.rs");
}
