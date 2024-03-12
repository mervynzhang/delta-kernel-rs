use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, PathArguments, Type};

#[proc_macro_derive(Schema, attributes(schema))]
pub fn derive_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;

    let schema_fields = gen_schema_fields(&input.data);
    let output = quote! {
        impl crate::actions::schemas::GetField for #struct_ident {
            fn get_field(name: impl Into<String>) -> crate::schema::StructField {
                use crate::actions::schemas::GetField;
                crate::schema::StructField::new(
                    name,
                    crate::schema::StructType::new(vec![
                        #schema_fields
                    ]),
                    // TODO: Ensure correct. By default not nullable, only can be made nullable by
                    // being wrapped in an Option
                    false,
                )
            }
        }
    };
    proc_macro::TokenStream::from(output)
}

// turn our struct name into the schema name, goes from snake_case to camelCase
fn get_schema_name(name: &Ident) -> Ident {
    let snake_name = name.to_string();
    let mut next_caps = false;
    let ret: String = snake_name
        .chars()
        .filter_map(|c| {
            if c == '_' {
                next_caps = true;
                None
            } else if next_caps {
                next_caps = false;
                // This assumes we're using ascii, should be okay
                Some(c.to_ascii_uppercase())
            } else {
                Some(c)
            }
        })
        .collect();
    Ident::new(&ret, name.span())
}

fn gen_schema_fields(data: &Data) -> TokenStream {
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };

    let schema_fields = fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap(); // we know these are named fields
        let name = get_schema_name(name);
        match field.ty {
            Type::Path(ref type_path) => {
                if let Some(fin) = type_path.path.segments.iter().last() {
                    let type_ident = &fin.ident;
                    if let PathArguments::AngleBracketed(angle_args) = &fin.arguments {
                        quote_spanned! {field.span()=>
                                        #type_ident::#angle_args::get_field(stringify!(#name))
                        }
                    } else {
                        quote_spanned! {field.span()=>
                                        #type_ident::get_field(stringify!(#name))
                        }
                    }
                } else {
                    panic!("Couldn't get type");
                }
            }
            _ => {
                panic!("Can't handle type: {:?}", field.ty);
            }
        }
    });
    quote! { #(#schema_fields),* }
}
