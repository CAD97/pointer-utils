fn main() {
    let cfg = autocfg::new();
    println!("cargo::rustc-check-cfg=cfg(has_strict_provenance)");
    cfg.emit_type_cfg("!", "has_never");
    cfg.emit_expression_cfg("<*const ()>::addr", "has_strict_provenance");
    autocfg::rerun_path("build.rs");
}
