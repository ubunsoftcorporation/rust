use proc_macro2::{self, Ident};
use quote::quote;
use syn::{self, parse_quote};

struct Attributes {
    ignore: bool,
    project: Option<Ident>,
}

fn parse_attributes(field: &syn::Field) -> Attributes {
    let mut attrs = Attributes { ignore: false, project: None };
    for attr in &field.attrs {
        let meta = &attr.meta;
        if !meta.path().is_ident("stable_hasher") {
            continue;
        }
        let mut any_attr = false;
        let _ = attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("ignore") {
                attrs.ignore = true;
                any_attr = true;
            }
            if nested.path.is_ident("project") {
                let _ = nested.parse_nested_meta(|meta| {
                    if attrs.project.is_none() {
                        attrs.project = meta.path.get_ident().cloned();
                    }
                    any_attr = true;
                    Ok(())
                });
            }
            Ok(())
        });
        if !any_attr {
            panic!("error parsing stable_hasher");
        }
    }
    attrs
}

pub(crate) fn hash_stable_generic_derive(
    mut s: synstructure::Structure<'_>,
) -> proc_macro2::TokenStream {
    let generic: syn::GenericParam = parse_quote!(__CTX);
    s.add_bounds(synstructure::AddBounds::Generics);
    s.add_impl_generic(generic);
    s.add_where_predicate(parse_quote! { __CTX: crate::HashStableContext });

    let discriminant = hash_stable_discriminant(&mut s);
    let body = hash_stable_body(&mut s);

    s.bound_impl(
        quote!(::rustc_data_structures::stable_hasher::HashStable<__CTX>),
        quote! {
            #[inline]
            fn hash_stable(
                &self,
                __hcx: &mut __CTX,
                __hasher: &mut ::rustc_data_structures::stable_hasher::StableHasher) {
                #discriminant
                match *self { #body }
            }
        },
    )
}

pub(crate) fn hash_stable_no_context_derive(
    mut s: synstructure::Structure<'_>,
) -> proc_macro2::TokenStream {
    let generic: syn::GenericParam = parse_quote!(__CTX);
    s.add_bounds(synstructure::AddBounds::Fields);
    s.add_impl_generic(generic);
    let body = s.each(|bi| {
        let attrs = parse_attributes(bi.ast());
        if attrs.ignore {
            quote! {}
        } else if let Some(project) = attrs.project {
            quote! {
                (&#bi.#project).hash_stable(__hcx, __hasher);
            }
        } else {
            quote! {
                #bi.hash_stable(__hcx, __hasher);
            }
        }
    });

    let discriminant = match s.ast().data {
        syn::Data::Enum(_) => quote! {
            ::std::mem::discriminant(self).hash_stable(__hcx, __hasher);
        },
        syn::Data::Struct(_) => quote! {},
        syn::Data::Union(_) => panic!("cannot derive on union"),
    };

    s.bound_impl(
        quote!(::rustc_data_structures::stable_hasher::HashStable<__CTX>),
        quote! {
            #[inline]
            fn hash_stable(
                &self,
                __hcx: &mut __CTX,
                __hasher: &mut ::rustc_data_structures::stable_hasher::StableHasher) {
                #discriminant
                match *self { #body }
            }
        },
    )
}

pub(crate) fn hash_stable_derive(mut s: synstructure::Structure<'_>) -> proc_macro2::TokenStream {
    let generic: syn::GenericParam = parse_quote!('__ctx);
    s.add_bounds(synstructure::AddBounds::Generics);
    s.add_impl_generic(generic);

    let discriminant = hash_stable_discriminant(&mut s);
    let body = hash_stable_body(&mut s);

    s.bound_impl(
        quote!(
            ::rustc_data_structures::stable_hasher::HashStable<
                ::rustc_query_system::ich::StableHashingContext<'__ctx>,
            >
        ),
        quote! {
            #[inline]
            fn hash_stable(
                &self,
                __hcx: &mut ::rustc_query_system::ich::StableHashingContext<'__ctx>,
                __hasher: &mut ::rustc_data_structures::stable_hasher::StableHasher) {
                #discriminant
                match *self { #body }
            }
        },
    )
}

fn hash_stable_discriminant(s: &mut synstructure::Structure<'_>) -> proc_macro2::TokenStream {
    match s.ast().data {
        syn::Data::Enum(_) => quote! {
            ::std::mem::discriminant(self).hash_stable(__hcx, __hasher);
        },
        syn::Data::Struct(_) => quote! {},
        syn::Data::Union(_) => panic!("cannot derive on union"),
    }
}

fn hash_stable_body(s: &mut synstructure::Structure<'_>) -> proc_macro2::TokenStream {
    s.each(|bi| {
        let attrs = parse_attributes(bi.ast());
        if attrs.ignore {
            quote! {}
        } else if let Some(project) = attrs.project {
            quote! {
                (&#bi.#project).hash_stable(__hcx, __hasher);
            }
        } else {
            quote! {
                #bi.hash_stable(__hcx, __hasher);
            }
        }
    })
}
