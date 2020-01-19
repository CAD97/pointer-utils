use std::env;

fn main() {
    let cfg = autocfg::new();
    if let Ok(yolo) = env::var("YOLO_RC_LAYOUT_KNOWN") {
        if !yolo.is_empty() && &yolo != "0" {
            autocfg::emit("yolo_rc_layout_known")
        }
    }
    cfg.emit_expression_cfg(
        "std::ptr::slice_from_raw_parts_mut",
        "has_ptr_slice_from_raw_parts",
    );

    autocfg::rerun_env("YOLO_RC_LAYOUT_KNOWN");
    autocfg::rerun_path("build.rs");
}
