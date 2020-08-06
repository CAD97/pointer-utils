use {
    proc_macro2::{Span, TokenStream},
    quote::{quote, quote_spanned},
    syn::spanned::Spanned,
};

#[no_mangle] // NB we set panic=abort and watt captures panics (somehow)
pub extern "C" fn derive_slice_dst(input: TokenStream) -> TokenStream {
    syn::parse2(input)
        .and_then(actually_derive_slice_dst)
        .unwrap_or_else(|err| err.to_compile_error())
}

fn actually_derive_slice_dst(
    syn::DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    }: syn::DeriveInput,
) -> syn::Result<TokenStream> {
    let data = match data {
        syn::Data::Enum(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "cannot implement `SliceDst` for enum",
            ))
        }
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "cannot implement `SliceDst` for union",
            ))
        }
        syn::Data::Struct(data) => data,
    };

    #[allow(non_snake_case)]
    let reprC = syn::parse_quote!(repr(C));
    if !attrs
        .iter()
        .flat_map(syn::Attribute::parse_meta)
        .any(|meta| meta == reprC)
    {
        return Err(syn::Error::new(
            Span::call_site(),
            "cannot derive `SliceDst` for non-`#[repr(C)]` struct",
        ));
    }

    let (head_field_ty, tail_field_ty) = {
        let mut fields: Vec<_> = data.fields.iter().map(|field| &field.ty).collect();
        match fields.pop() {
            Some(tail) => (fields, tail),
            None => {
                return Err(syn::Error::new_spanned(
                    data.fields,
                    "cannot implement `SliceDst` for struct with no fields",
                ))
            }
        }
    };

    let tail_layout = quote_spanned! {tail_field_ty.span()=>
        <#tail_field_ty as SliceDst>::layout_for(len)
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        #[allow(unsafe_code)]
        unsafe impl #impl_generics SliceDst for #ident #ty_generics #where_clause {
            fn layout_for(len: usize) -> ::core::alloc::Layout {
                let mut layout = ::core::alloc::Layout::new::<()>();
                const err_msg: &'static str = concat!("too big `", stringify!(#ident), "` requested from `SliceDst::layout_for`");
                #(
                    layout = layout.extend(::core::alloc::Layout::new::<#head_field_ty>()).expect(err_msg).0;
                )*
                layout = layout.extend(#tail_layout).expect(err_msg).0;
                layout.pad_to_align()
            }

            fn retype(ptr: ::core::ptr::NonNull<[()]>) -> ::core::ptr::NonNull<Self> {
                unsafe { ::core::ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
            }
        }
    })
}
