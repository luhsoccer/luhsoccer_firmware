use proc_macro2::Literal;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Field, Ident,
    Type::Path, TypePath,
};

fn get_attribute<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attributes
        .iter()
        .find(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == name)
}

fn get_attribute_needed<'a>(attributes: &'a [Attribute], name: &str) -> &'a Attribute {
    get_attribute(attributes, name).unwrap_or_else(|| panic!("attribute {name} is needed"))
}

#[derive(Debug)]
struct MyVariant {
    name: Ident,
    value: u32,
}

/// generates `TMC4671Field` impl for enums
///
/// # Panics
///
/// Panics if input is not a enum or is missing needed attributes
#[proc_macro_derive(TMC4671Field, attributes(val))]
pub fn derive_tmc4671field(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics: _,
        data,
    } = parse_macro_input!(input as DeriveInput);

    let vs: Vec<_> = match data {
        Data::Enum(DataEnum {
            enum_token: _,
            brace_token: _,
            variants,
        }) => {
            let mut next_value = 0;
            variants
                .iter()
                .map(|v| {
                    let value = get_attribute(&v.attrs, "val").map_or(next_value, |a| {
                        a.parse_args::<Literal>()
                            .expect("Value needs to be given in the offset")
                            .to_string()
                            .parse::<u32>()
                            .expect("unable to parse literal into u32")
                    });
                    next_value = value + 1;
                    let name = v.ident.clone();
                    MyVariant { name, value }
                })
                .collect()
        }
        _ => panic!("only enums supportet for now"),
    };

    let serialize_variants = vs.iter().fold(quote! {}, |ts, MyVariant { name, value }| {
        quote! {
            #ts
            Self::#name => #value,
        }
    });

    let deserialize_variants = vs.iter().fold(quote! {}, |ts, MyVariant { name, value }| {
        quote! {
            #ts
            #value => Ok(Self::#name),
        }
    });

    quote! {
        impl TMC4671Field for #ident {
            fn serialize_field<const OFFSET: u8, const SIZE: u8>(&self, buffer: &mut [u8; 5]) {
                match self {
                    #serialize_variants
                }
                .serialize_field::<OFFSET, SIZE>(buffer);
            }

            fn deserialize_field<const OFFSET: u8, const SIZE: u8>(input: [u8; 4]) -> Result<Self, Error> {
                let int = u32::deserialize_field::<OFFSET, SIZE>(input)?;
                match int {
                    #deserialize_variants
                    _ => Err(Error::InvalidState),
                }
            }
        }
    }
    .into()
}

#[derive(Debug)]
struct MyField {
    name: Ident,
    ty: Ident,
    offset: proc_macro2::TokenStream,
    size: proc_macro2::TokenStream,
}

impl MyField {
    fn from_field(
        field: &Field,
        offset: &proc_macro2::TokenStream,
    ) -> (Self, proc_macro2::TokenStream) {
        let name = field.ident.clone().expect("need named fields");
        let ty = if let Path(TypePath { qself: _, path }) = &field.ty {
            path.get_ident()
        } else {
            None
        }
        .expect("need type of field")
        .clone();
        let offset = get_attribute(&field.attrs, "offset").map_or(offset.clone(), |a| {
            let ts: proc_macro2::TokenStream = a
                .parse_args()
                .expect("offset needs to be given in the attribute");
            quote! {#ts}
        });
        let size = get_attribute(&field.attrs, "size").map_or_else(
            || {
                if ty == "bool" {
                    quote! {1}
                } else {
                    quote! {{::core::mem::size_of::<#ty>() as u8 * 8}}
                }
            },
            |a| {
                let ts: proc_macro2::TokenStream = a
                    .parse_args()
                    .expect("size needs to be given in the attribute");
                quote! {#ts}
            },
        );
        let new_offset = quote! {#offset + #size};
        (
            Self {
                name,
                ty,
                offset,
                size,
            },
            new_offset,
        )
    }
}

/// Derive ``TMC4671Command`` for a struckt where all members implement ``TMC4671Field``
///
/// # Panics
///
/// Panics if the input is not a struct or attributes are missing
#[proc_macro_derive(TMC4671Command, attributes(addr, readonly, offset, size))]
pub fn derive_tmc4671command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        attrs,
        vis: _,
        ident,
        generics: _,
        data,
    } = parse_macro_input!(input as DeriveInput);

    let addr_attr = get_attribute_needed(&attrs, "addr");
    let readonly = get_attribute(&attrs, "readonly").is_some();
    let write = !readonly;
    let addr: proc_macro2::TokenStream = addr_attr
        .parse_args()
        .expect("address needs to be given in the attribute");

    let fields: Vec<_> = match &data {
        Data::Struct(DataStruct {
            struct_token: _,
            fields,
            semi_token: _,
        }) => {
            let mut next_offset = quote! {0};
            fields
                .iter()
                .map(|f| {
                    let (field, offset) = MyField::from_field(f, &next_offset);
                    next_offset = offset;
                    field
                })
                .collect()
        }
        _ => panic!("only structs can be used for now"),
    };

    let write_impl = if write {
        let fields_serialization = fields.iter().fold(
            quote! {},
            |ts,
             MyField {
                 name,
                 ty: _,
                 offset,
                 size,
             }| {
                quote! {#ts self.#name.serialize_field::<{#offset}, #size>(&mut o);}
            },
        );
        quote! {
            impl TMC4671WriteCommand for #ident {
                fn serialize_write(&self) -> [u8; 5] {
                    let mut o = [#addr | 0b10000000, 0, 0, 0, 0];
                    #fields_serialization
                    o
                }
            }
        }
    } else {
        quote! {}
    };

    let read_impl = {
        let fields_deserialisation = fields.iter().fold(
            quote! {},
            |ts,
             MyField {
                 name,
                 ty,
                 offset,
                 size,
             }| {
                quote! {
                    #ts
                    #name: #ty::deserialize_field::<{#offset}, #size>(input)?,
                }
            },
        );
        quote! {
            impl TMC4671Command for #ident {
                fn serialize_read() -> u8 {
                    #addr
                }

                fn deserialize(input: [u8; 4]) -> Result<Self, Error> {
                    Ok(Self {
                        #fields_deserialisation
                    })
                }
            }
        }
    };

    quote! {
        #write_impl

        #read_impl
    }
    .into()
}
