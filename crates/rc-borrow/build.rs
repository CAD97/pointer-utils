fn main() {
    let cfg = autocfg::new();
    cfg.emit_type_cfg("std::sync::Arc::as_raw", "has_Arc__into_raw");
    cfg.emit_type_cfg("std::sync::Arc::clone_raw", "has_Arc__clone_raw");
    cfg.emit_type_cfg("std::rc::Rc::as_raw", "has_Rc__into_raw");
    cfg.emit_type_cfg("std::rc::Rc::clone_raw", "has_Rc__clone_raw");
    autocfg::rerun_path("build.rs");
}
