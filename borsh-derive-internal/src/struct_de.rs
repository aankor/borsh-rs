use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Fields, Ident, ItemStruct, Path, WhereClause};

use crate::attribute_helpers::{
    contains_deserialize_with, contains_initialize_with, contains_skip,
};

pub fn struct_de(input: &ItemStruct, cratename: Ident) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.map_or_else(
        || WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        },
        Clone::clone,
    );
    let init_method = contains_initialize_with(&input.attrs)?;
    let return_value = match &input.fields {
        Fields::Named(fields) => {
            let mut body = TokenStream2::new();
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let delta = if contains_skip(&field.attrs) {
                    quote! {
                        #field_name: Default::default(),
                    }
                } else {
                    let deserialize_with = contains_deserialize_with(&field.attrs)?;
                    let deserializer: Path = deserialize_with
                        .clone()
                        .unwrap_or(syn::parse_quote! { #cratename::BorshDeserialize::deserialize });

                    if deserialize_with.is_none() {
                        let field_type = &field.ty;
                        where_clause.predicates.push(
                            syn::parse2(quote! {
                                #field_type: #cratename::BorshDeserialize
                            })
                            .unwrap(),
                        );
                    }

                    quote! {
                        #field_name: #deserializer(buf)?,
                    }
                };
                body.extend(delta);
            }
            quote! {
                Self { #body }
            }
        }
        Fields::Unnamed(fields) => {
            let mut body = TokenStream2::new();
            for field in fields.unnamed.iter() {
                let deserialize_with = contains_deserialize_with(&field.attrs)?;
                let deserializer: Path = deserialize_with
                    .unwrap_or(syn::parse_quote! { #cratename::BorshDeserialize::deserialize });

                let delta = quote! {
                    #deserializer(buf)?,
                };
                body.extend(delta);
            }
            quote! {
                Self( #body )
            }
        }
        Fields::Unit => {
            quote! {
                Self {}
            }
        }
    };
    if let Some(method_ident) = init_method {
        Ok(quote! {
            impl #impl_generics #cratename::de::BorshDeserialize for #name #ty_generics #where_clause {
                fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, #cratename::maybestd::io::Error> {
                    let mut return_value = #return_value;
                    return_value.#method_ident();
                    Ok(return_value)
                }
            }
        })
    } else {
        Ok(quote! {
            impl #impl_generics #cratename::de::BorshDeserialize for #name #ty_generics #where_clause {
                fn deserialize(buf: &mut &[u8]) -> ::core::result::Result<Self, #cratename::maybestd::io::Error> {
                    Ok(#return_value)
                }
            }
        })
    }
}
