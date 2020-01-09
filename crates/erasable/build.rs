fn main() {
    let cfg = autocfg::new();
    cfg.emit_expression_cfg("{ extern { type T; } () }", "has_extern_type");
    cfg.emit_type_cfg("!", "has_never");
    autocfg::rerun_path("build.rs");
}
