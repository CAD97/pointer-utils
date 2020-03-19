fn main() {
    let cfg = autocfg::new();
    cfg.emit_expression_cfg(
        "std::ptr::slice_from_raw_parts_mut::<()>",
        "has_ptr_slice_from_raw_parts",
    );

    autocfg::rerun_path("build.rs");
}
