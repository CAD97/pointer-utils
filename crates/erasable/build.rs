use std::env;

fn main() {
    let cfg = autocfg::new();

    cfg.emit_expression_cfg("{ extern { type T; } () }", "has_extern_type");
    // NB: Requires this impl to cover `T: ?Sized`, which is not the case as of 2020-09-01.
    // cfg.emit_type_cfg("std::sync::Weak::into_raw", "has_Weak__into_raw");
    cfg.emit_type_cfg("!", "has_never");

    if let Ok(var) = env::var("ERASABLE_ENFORCE_1_1_0_SEMANTICS") {
        if !var.is_empty() && var != "0" {
            autocfg::emit("enforce_1_1_0_semantics");
        }
    }

    autocfg::rerun_env("ERASABLE_ENFORCE_1_1_0_SEMANTICS");
    autocfg::rerun_path("build.rs");
}
