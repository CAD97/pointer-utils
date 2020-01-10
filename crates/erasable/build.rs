fn main() {
    let cfg = autocfg::new();
    cfg.emit_expression_cfg("{ extern { type T; } () }", "has_extern_type");
    // NB: Requires this impl to cover `T: ?Sized`, which is not the case as of 2020-09-01.
    // cfg.emit_type_cfg("std::sync::Weak::into_raw", "has_Weak__into_raw");
    cfg.emit_type_cfg("!", "has_never");
    autocfg::rerun_path("build.rs");
}
