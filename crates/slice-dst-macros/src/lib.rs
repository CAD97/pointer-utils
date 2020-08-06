use {proc_macro::TokenStream, watt::WasmMacro};

static MACRO: WasmMacro = WasmMacro::new(WASM);
static WASM: &[u8] = include_bytes!("slice_dst_macros_impl.wasm");

#[proc_macro_derive(SliceDst)]
pub fn derive_slice_dst(input: TokenStream) -> TokenStream {
    // panic!("{}",
    MACRO.proc_macro("derive_slice_dst", input)
    // .to_string())
}
