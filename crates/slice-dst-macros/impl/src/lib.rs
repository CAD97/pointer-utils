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

#[derive(Default)]
struct SliceDstMeta {
    new_from_iter: Option<syn::Ident>,
    new_from_slice: Option<syn::Ident>,
}

mod kw {
    syn::custom_keyword!(new_from_iter);
    syn::custom_keyword!(new_from_slice);
}

impl syn::parse::Parse for SliceDstMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let _ = syn::parenthesized!(content in input);
        let mut this = SliceDstMeta::default();
        while !content.is_empty() {
            let la = content.lookahead1();

            if la.peek(kw::new_from_iter) {
                if this.new_from_iter.is_some() {
                    return Err(content.error("duplicate `new_from_iter`"));
                }
                let ident = content.parse()?;
                if content.peek(syn::Token![=]) {
                    let _: syn::Token![=] = content.parse()?;
                    let ident = content.parse()?;
                    this.new_from_iter = Some(ident);
                } else {
                    this.new_from_iter = Some(ident);
                }
            } else if la.peek(kw::new_from_slice) {
                if this.new_from_slice.is_some() {
                    return Err(content.error("duplicate `new_from_slice`"));
                }
                let ident = content.parse()?;
                if content.peek(syn::Token![=]) {
                    let _: syn::Token![=] = content.parse()?;
                    let ident = content.parse()?;
                    this.new_from_slice = Some(ident);
                } else {
                    this.new_from_slice = Some(ident);
                }
            } else {
                return Err(la.error());
            }

            if content.peek(syn::Token![,]) {
                let _: syn::Token![,] = content.parse()?;
            } else {
                break;
            }
        }
        Ok(this)
    }
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

    let mut saw_repr_c = false;
    let mut new_from_iter = None;
    let mut new_from_slice = None;

    for attr in attrs.into_iter() {
        if attr.path.is_ident("repr") {
            if let Ok(meta) = attr.parse_meta() {
                saw_repr_c = saw_repr_c || meta == reprC;
            }
        } else if attr.path.is_ident("slice_dst") {
            let meta: SliceDstMeta = syn::parse2(attr.tokens)?;

            if new_from_iter.is_some() && meta.new_from_iter.is_some() {
                return Err(syn::Error::new(
                    meta.new_from_iter.unwrap().span(),
                    "duplicate `new_from_iter`",
                ));
            } else {
                new_from_iter = meta.new_from_iter;
            }
            if new_from_slice.is_some() && meta.new_from_slice.is_some() {
                return Err(syn::Error::new(
                    meta.new_from_slice.unwrap().span(),
                    "duplicate `new_from_iter`",
                ));
            } else {
                new_from_slice = meta.new_from_slice;
            }
        }
    }

    if !saw_repr_c {
        return Err(syn::Error::new(
            Span::call_site(),
            "cannot derive `SliceDst` for non-`#[repr(C)]` struct",
        ));
    }

    let (head_field_tys, tail_field_ty) = {
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
    let mut output_stream = quote! {
        #[allow(unsafe_code)]
        unsafe impl #impl_generics SliceDst for #ident #ty_generics #where_clause {
            fn layout_for(len: usize) -> ::core::alloc::Layout {
                let mut layout = ::core::alloc::Layout::new::<()>();
                const err_msg: &'static str = concat!("too big `", stringify!(#ident), "` requested from `SliceDst::layout_for`");
                #(
                    layout = layout.extend(::core::alloc::Layout::new::<#head_field_tys>()).expect(err_msg).0;
                )*
                layout = layout.extend(#tail_layout).expect(err_msg).0;
                layout.pad_to_align()
            }

            fn retype(ptr: ::core::ptr::NonNull<[()]>) -> ::core::ptr::NonNull<Self> {
                unsafe { ::core::ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
            }
        }
    };

    if new_from_iter.is_some() || new_from_slice.is_some() {
        let tail_field_item_ty = match tail_field_ty {
            syn::Type::Slice(ty) => &*ty.elem,
            ty => {
                return Err(syn::Error::new(
                    ty.span(),
                    "tail type must be a slice to derive a slice_dst constructor",
                ))
            }
        };

        let sized_type_count = head_field_tys.len();
        let sized_type_index: Vec<syn::Index> = (0..sized_type_count).map(Into::into).collect();

        if let Some(new_from_slice) = new_from_slice {
            output_stream.extend(quote! {
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[allow(clippy::new_ret_no_self)]
                    /// Create a new instance of this slice dst by copying a tail slice.
                    fn #new_from_slice<A>(sized: (#(#head_field_tys,)*), slice: &[#tail_field_item_ty]) -> A
                    where
                        A: ::slice_dst::AllocSliceDst<Self>,
                        #tail_field_item_ty: ::core::marker::Copy,
                    {
                        let len = slice.len();
                        let mut layout = ::core::alloc::Layout::new::<()>();
                        const err_msg: &'static str = concat!("too big `", stringify!(#ident), "` requested from `", stringify!(#ident), "::", stringify!(#new_from_slice), "`");
                        #[allow(clippy::eval_order_dependence)]
                        let offsets: [usize; #sized_type_count + 1] = [
                            #({
                                let (extended, offset) = layout.extend(::core::alloc::Layout::new::<#head_field_tys>()).expect(err_msg);
                                layout = extended;
                                offset
                            },)*
                            {
                                let (extended, offset) = layout.extend(#tail_layout).expect(err_msg);
                                layout = extended.pad_to_align();
                                offset
                            },
                        ];

                        unsafe {
                            A::new_slice_dst(len, |ptr| {
                                let raw = ptr.as_ptr().cast::<u8>();
                                #(
                                    ::core::ptr::write(raw.add(offsets[#sized_type_index]).cast(), sized.#sized_type_index);
                                )*
                                ::core::ptr::copy_nonoverlapping(slice.as_ptr(), raw.add(offsets[#sized_type_count]).cast(), len);
                                debug_assert_eq!(::core::alloc::Layout::for_value(ptr.as_ref()), layout);
                            })
                        }
                    }
                }
            });
        }

        if let Some(new_from_iter) = new_from_iter {
            output_stream.extend(quote! {
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[allow(clippy::new_ret_no_self)]
                    /// Create a new instance of this slice dst by collecting from a tail iterator.
                    pub fn #new_from_iter<A, I>(sized: (#(#head_field_tys,)*), iter: I) -> A
                    where
                        A: ::slice_dst::AllocSliceDst<Self>,
                        I: ::core::iter::IntoIterator<Item = #tail_field_item_ty>,
                        I::IntoIter: ::core::iter::ExactSizeIterator,
                    {
                        let mut iter = iter.into_iter();
                        let len = iter.len();
                        let mut layout = ::core::alloc::Layout::new::<()>();
                        const err_msg: &'static str = concat!("too big `", stringify!(#ident), "` requested from `", stringify!(#ident), "::", stringify!(#new_from_iter), "`");
                        #[allow(clippy::eval_order_dependence)]
                        let offsets: [usize; #sized_type_count + 1] = [
                            #({
                                let (extended, offset) = layout.extend(::core::alloc::Layout::new::<#head_field_tys>()).expect(err_msg);
                                layout = extended;
                                offset
                            },)*
                            {
                                let (extended, offset) = layout.extend(#tail_layout).expect(err_msg);
                                layout = extended.pad_to_align();
                                offset
                            },
                        ];

                        struct SliceWriter<Item> {
                            ptr: ::core::ptr::NonNull<Item>,
                            len: usize,
                        }

                        impl<Item> ::core::ops::Drop for SliceWriter<Item> {
                            fn drop(&mut self) {
                                unsafe {
                                    ::core::ptr::drop_in_place(::core::ptr::slice_from_raw_parts_mut(
                                        self.ptr.as_ptr(),
                                        self.len,
                                    ))
                                }
                            }
                        }

                        impl<Item> SliceWriter<Item> {
                            unsafe fn new(ptr: *mut Item) -> Self {
                                SliceWriter {
                                    ptr: ::core::ptr::NonNull::new_unchecked(ptr),
                                    len: 0,
                                }
                            }

                            unsafe fn push(&mut self, item: Item) {
                                self.ptr.as_ptr().add(self.len).write(item);
                                self.len += 1;
                            }
                        }

                        unsafe {
                            A::new_slice_dst(len, move |ptr| {
                                let raw = ptr.as_ptr().cast::<u8>();
                                let mut slice_writer = SliceWriter::new(raw.add(offsets[#sized_type_count]).cast());
                                for _ in 0..len {
                                    slice_writer.push(iter.next().expect("`ExactSizeIterator` over-reported length"));
                                }
                                assert!(iter.next().is_none(), "`ExactSizeIterator` under-reported length");
                                ::core::mem::forget(slice_writer);
                                #(
                                    ::core::ptr::write(raw.add(offsets[#sized_type_index]).cast(), sized.#sized_type_index);
                                )*
                                debug_assert_eq!(::core::alloc::Layout::for_value(ptr.as_ref()), layout);
                            })
                        }
                    }
                }
            });
        }
    }

    Ok(output_stream)
}
