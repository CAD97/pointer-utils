fn main() {
    let cfg = autocfg::new();
    cfg.emit_type_cfg("!", "has_never");
    autocfg::rerun_path("build.rs");
}
