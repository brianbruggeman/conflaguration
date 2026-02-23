use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Lit;
use syn::Meta;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

#[proc_macro_derive(Settings, attributes(settings, setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    match derive_settings_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(Validate, attributes(settings, setting))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    match derive_validate_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(ConfigDisplay, attributes(settings, setting))]
pub fn derive_config_display(input: TokenStream) -> TokenStream {
    match derive_config_display_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct StructAttrs {
    prefix: Option<String>,
}

struct FieldAttrs {
    envs: Vec<String>,
    envs_override: bool,
    default: Option<Lit>,
    default_str: Option<String>,
    nested: bool,
    skip: bool,
    sensitive: bool,
    override_prefix: Option<Option<String>>,
}

struct BracketedStrings {
    values: Vec<String>,
}

impl Parse for BracketedStrings {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let lits: Punctuated<syn::LitStr, Token![,]> = content.parse_terminated(|input| input.parse::<syn::LitStr>(), Token![,])?;
        Ok(Self {
            values: lits.iter().map(syn::LitStr::value).collect(),
        })
    }
}

fn parse_struct_attrs(input: &DeriveInput) -> syn::Result<StructAttrs> {
    let mut prefix = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("settings") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("prefix") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                prefix = Some(lit.value());
                return Ok(());
            }
            Err(meta.error("unknown settings attribute"))
        })?;
    }
    Ok(StructAttrs { prefix })
}

fn parse_env_list(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<Vec<String>> {
    let value = meta.value()?;
    if value.peek(syn::token::Bracket) {
        let parsed: BracketedStrings = value.parse()?;
        Ok(parsed.values)
    } else {
        let lit: syn::LitStr = value.parse()?;
        Ok(vec![lit.value()])
    }
}

fn parse_field_attrs(field: &syn::Field) -> syn::Result<FieldAttrs> {
    let mut attrs = FieldAttrs {
        envs: Vec::new(),
        envs_override: false,
        default: None,
        default_str: None,
        nested: false,
        skip: false,
        sensitive: false,
        override_prefix: None,
    };

    for attr in &field.attrs {
        if !attr.path().is_ident("setting") {
            continue;
        }

        if let Meta::List(_) = &attr.meta {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("envs") {
                    attrs.envs = parse_env_list(&meta)?;
                    return Ok(());
                }
                if meta.path.is_ident("r#override") || meta.path.is_ident("override") {
                    attrs.envs_override = true;
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    attrs.default = Some(lit);
                    return Ok(());
                }
                if meta.path.is_ident("default_str") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    attrs.default_str = Some(lit.value());
                    return Ok(());
                }
                if meta.path.is_ident("nested") {
                    attrs.nested = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip") {
                    attrs.skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("sensitive") {
                    attrs.sensitive = true;
                    return Ok(());
                }
                if meta.path.is_ident("override_prefix") {
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?;
                        let lit: syn::LitStr = value.parse()?;
                        attrs.override_prefix = Some(Some(lit.value()));
                    } else {
                        attrs.override_prefix = Some(None);
                    }
                    return Ok(());
                }
                Err(meta.error("unknown setting attribute"))
            })?;
        }
    }

    Ok(attrs)
}

fn field_name_to_env_key(name: &str) -> String {
    name.to_uppercase()
}

fn build_key_list(prefix: &Option<String>, field_name: &str, attrs: &FieldAttrs) -> Vec<String> {
    let mut keys = Vec::new();

    let names = if attrs.envs.is_empty() { vec![field_name_to_env_key(field_name)] } else { attrs.envs.clone() };

    for name in &names {
        let key = if attrs.envs_override {
            name.clone()
        } else {
            match prefix {
                Some(pfx) => format!("{pfx}_{name}"),
                None => name.clone(),
            }
        };
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    keys
}

fn gen_resolve_call(keys_expr: TokenStream2, attrs: &FieldAttrs) -> TokenStream2 {
    if let Some(lit) = &attrs.default {
        if matches!(lit, Lit::Str(_)) {
            let lit_str = match lit {
                Lit::Str(strlit) => strlit.value(),
                _ => unreachable!(),
            };
            return quote! {
                ::conflaguration::resolve_or_parse(#keys_expr, #lit_str)?
            };
        }
        return quote! {
            ::conflaguration::resolve_or(#keys_expr, #lit)?
        };
    }

    if let Some(default_str) = &attrs.default_str {
        return quote! {
            ::conflaguration::resolve_or_parse(#keys_expr, #default_str)?
        };
    }

    quote! {
        ::conflaguration::resolve(#keys_expr)?
    }
}

fn gen_field_expr(field: &syn::Field, prefix: &Option<String>) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let field_name_str = field_name.to_string();
    let attrs = parse_field_attrs(field)?;

    if attrs.skip {
        return Ok(quote! { ::core::default::Default::default() });
    }

    if attrs.nested {
        let field_type = &field.ty;
        return match &attrs.override_prefix {
            Some(None) => match prefix {
                Some(pfx) => Ok(quote! {
                    {
                        let __nested = match <#field_type as ::conflaguration::Settings>::PREFIX {
                            Some(__inner) => ::std::format!("{}_{}", #pfx, __inner),
                            None => #pfx.to_string(),
                        };
                        <#field_type as ::conflaguration::Settings>::from_env_with_prefix(&__nested)?
                    }
                }),
                None => Ok(quote! {
                    <#field_type as ::conflaguration::Settings>::from_env()?
                }),
            },
            Some(Some(explicit)) => Ok(quote! {
                <#field_type as ::conflaguration::Settings>::from_env_with_prefix(#explicit)?
            }),
            None => Ok(quote! {
                <#field_type as ::conflaguration::Settings>::from_env()?
            }),
        };
    }

    let keys = build_key_list(prefix, &field_name_str, &attrs);
    let keys_ref = &keys;
    let keys_expr = quote! { &[#(#keys_ref),*] };
    Ok(gen_resolve_call(keys_expr, &attrs))
}

fn gen_field_expr_dynamic(field: &syn::Field) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let field_name_str = field_name.to_string();
    let attrs = parse_field_attrs(field)?;

    if attrs.skip {
        return Ok(quote! { ::core::default::Default::default() });
    }

    if attrs.nested {
        let field_type = &field.ty;
        return match &attrs.override_prefix {
            Some(None) => Ok(quote! {
                {
                    let __nested = match <#field_type as ::conflaguration::Settings>::PREFIX {
                        Some(__inner) => ::std::format!("{}_{}", __prefix, __inner),
                        None => __prefix.to_string(),
                    };
                    <#field_type as ::conflaguration::Settings>::from_env_with_prefix(&__nested)?
                }
            }),
            Some(Some(explicit)) => Ok(quote! {
                <#field_type as ::conflaguration::Settings>::from_env_with_prefix(#explicit)?
            }),
            None => Ok(quote! {
                <#field_type as ::conflaguration::Settings>::from_env()?
            }),
        };
    }

    let names = if attrs.envs.is_empty() {
        vec![field_name_to_env_key(&field_name_str)]
    } else {
        attrs.envs.clone()
    };

    let names_ref = &names;
    let is_override = attrs.envs_override;
    let keys_build = if is_override {
        quote! {
            let __keys: Vec<String> = vec![#(#names_ref.to_string()),*];
        }
    } else {
        quote! {
            let __keys: Vec<String> = vec![#(::std::format!("{}_{}", __prefix, #names_ref)),*];
        }
    };

    let resolve = if let Some(lit) = &attrs.default {
        if matches!(lit, Lit::Str(_)) {
            let lit_str = match lit {
                Lit::Str(strlit) => strlit.value(),
                _ => unreachable!(),
            };
            quote! { ::conflaguration::resolve_or_parse(&__key_refs, #lit_str)? }
        } else {
            quote! { ::conflaguration::resolve_or(&__key_refs, #lit)? }
        }
    } else if let Some(default_str) = &attrs.default_str {
        quote! { ::conflaguration::resolve_or_parse(&__key_refs, #default_str)? }
    } else {
        quote! { ::conflaguration::resolve(&__key_refs)? }
    };

    Ok(quote! {
        {
            #keys_build
            let __key_refs: Vec<&str> = __keys.iter().map(|s| s.as_str()).collect();
            #resolve
        }
    })
}

fn gen_override_field_expr(field: &syn::Field, prefix: &Option<String>) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let field_name_str = field_name.to_string();
    let attrs = parse_field_attrs(field)?;

    if attrs.skip {
        return Ok(quote! {});
    }

    if attrs.nested {
        let field_type = &field.ty;
        return match &attrs.override_prefix {
            Some(None) => match prefix {
                Some(pfx) => Ok(quote! {
                    {
                        let __nested = match <#field_type as ::conflaguration::Settings>::PREFIX {
                            Some(__inner) => ::std::format!("{}_{}", #pfx, __inner),
                            None => #pfx.to_string(),
                        };
                        ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, &__nested)?;
                    }
                }),
                None => Ok(quote! {
                    ::conflaguration::Settings::override_from_env(&mut self.#field_name)?;
                }),
            },
            Some(Some(explicit)) => Ok(quote! {
                ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, #explicit)?;
            }),
            None => Ok(quote! {
                ::conflaguration::Settings::override_from_env(&mut self.#field_name)?;
            }),
        };
    }

    let keys = build_key_list(prefix, &field_name_str, &attrs);
    let keys_ref = &keys;
    let keys_expr = quote! { &[#(#keys_ref),*] };

    Ok(quote! {
        {
            let __keys: &[&str] = #keys_expr;
            if __keys.iter().any(|__k| ::std::env::var(__k).is_ok()) {
                self.#field_name = ::conflaguration::resolve(__keys)?;
            }
        }
    })
}

fn gen_override_field_expr_dynamic(field: &syn::Field) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let field_name_str = field_name.to_string();
    let attrs = parse_field_attrs(field)?;

    if attrs.skip {
        return Ok(quote! {});
    }

    if attrs.nested {
        let field_type = &field.ty;
        return match &attrs.override_prefix {
            Some(None) => Ok(quote! {
                {
                    let __nested = match <#field_type as ::conflaguration::Settings>::PREFIX {
                        Some(__inner) => ::std::format!("{}_{}", __prefix, __inner),
                        None => __prefix.to_string(),
                    };
                    ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, &__nested)?;
                }
            }),
            Some(Some(explicit)) => Ok(quote! {
                ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, #explicit)?;
            }),
            None => Ok(quote! {
                ::conflaguration::Settings::override_from_env(&mut self.#field_name)?;
            }),
        };
    }

    let names = if attrs.envs.is_empty() {
        vec![field_name_to_env_key(&field_name_str)]
    } else {
        attrs.envs.clone()
    };

    let names_ref = &names;
    let is_override = attrs.envs_override;
    let keys_build = if is_override {
        quote! {
            let __keys: Vec<String> = vec![#(#names_ref.to_string()),*];
        }
    } else {
        quote! {
            let __keys: Vec<String> = vec![#(::std::format!("{}_{}", __prefix, #names_ref)),*];
        }
    };

    Ok(quote! {
        {
            #keys_build
            let __key_refs: Vec<&str> = __keys.iter().map(|s| s.as_str()).collect();
            if __key_refs.iter().any(|__k| ::std::env::var(__k).is_ok()) {
                self.#field_name = ::conflaguration::resolve(&__key_refs)?;
            }
        }
    })
}

fn derive_settings_impl(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input: DeriveInput = syn::parse2(input)?;
    let struct_attrs = parse_struct_attrs(&input)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Err(syn::Error::new(input.ident.span(), "only named struct fields supported")),
        },
        _ => return Err(syn::Error::new(input.ident.span(), "Settings can only be derived on structs")),
    };

    let mut static_exprs = Vec::new();
    let mut dynamic_exprs = Vec::new();
    let mut override_static_stmts = Vec::new();
    let mut override_dynamic_stmts = Vec::new();
    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let static_expr = gen_field_expr(field, &struct_attrs.prefix)?;
        let dynamic_expr = gen_field_expr_dynamic(field)?;
        let override_static = gen_override_field_expr(field, &struct_attrs.prefix)?;
        let override_dynamic = gen_override_field_expr_dynamic(field)?;
        static_exprs.push(quote! { #field_name: #static_expr });
        dynamic_exprs.push(quote! { #field_name: #dynamic_expr });
        override_static_stmts.push(override_static);
        override_dynamic_stmts.push(override_dynamic);
    }

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let prefix_const = match &struct_attrs.prefix {
        Some(pfx) => quote! { const PREFIX: Option<&'static str> = Some(#pfx); },
        None => quote! { const PREFIX: Option<&'static str> = None; },
    };

    Ok(quote! {
        impl #impl_generics ::conflaguration::Settings for #struct_name #type_generics #where_clause {
            #prefix_const

            fn from_env() -> ::conflaguration::Result<Self> {
                Ok(Self {
                    #(#static_exprs),*
                })
            }

            fn from_env_with_prefix(__prefix: &str) -> ::conflaguration::Result<Self> {
                Ok(Self {
                    #(#dynamic_exprs),*
                })
            }

            fn override_from_env(&mut self) -> ::conflaguration::Result<()> {
                #(#override_static_stmts)*
                Ok(())
            }

            fn override_from_env_with_prefix(&mut self, __prefix: &str) -> ::conflaguration::Result<()> {
                #(#override_dynamic_stmts)*
                Ok(())
            }
        }
    })
}

fn derive_validate_impl(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input: DeriveInput = syn::parse2(input)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Err(syn::Error::new(input.ident.span(), "only named struct fields supported")),
        },
        _ => return Err(syn::Error::new(input.ident.span(), "Validate can only be derived on structs")),
    };

    let mut validate_calls = Vec::new();
    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let field_name_str = field_name.to_string();
        let attrs = parse_field_attrs(field)?;

        if attrs.nested {
            validate_calls.push(quote! {
                if let Err(__err) = ::conflaguration::Validate::validate(&self.#field_name) {
                    match __err {
                        ::conflaguration::Error::Validation { errors: __inner } => {
                            for mut __ve in __inner {
                                __ve.prepend_path(#field_name_str);
                                __errors.push(__ve);
                            }
                        }
                        __other => return Err(__other),
                    }
                }
            });
        }
    }

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    if validate_calls.is_empty() {
        return Ok(quote! {
            impl #impl_generics ::conflaguration::Validate for #struct_name #type_generics #where_clause {
                fn validate(&self) -> ::conflaguration::Result<()> {
                    Ok(())
                }
            }
        });
    }

    Ok(quote! {
        impl #impl_generics ::conflaguration::Validate for #struct_name #type_generics #where_clause {
            fn validate(&self) -> ::conflaguration::Result<()> {
                let mut __errors: Vec<::conflaguration::ValidationMessage> = vec![];
                #(#validate_calls)*
                if __errors.is_empty() {
                    Ok(())
                } else {
                    Err(::conflaguration::Error::Validation { errors: __errors })
                }
            }
        }
    })
}

fn derive_config_display_impl(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input: DeriveInput = syn::parse2(input)?;
    let struct_attrs = parse_struct_attrs(&input)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Err(syn::Error::new(input.ident.span(), "only named struct fields supported")),
        },
        _ => return Err(syn::Error::new(input.ident.span(), "ConfigDisplay can only be derived on structs")),
    };

    let mut display_lines = Vec::new();
    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let field_name_str = field_name.to_string();
        let attrs = parse_field_attrs(field)?;

        if attrs.skip {
            display_lines.push(quote! {
                ::std::writeln!(__f, "{}{} = {:?} (skipped)", __indent, #field_name_str, self.#field_name)?;
            });
            continue;
        }

        if attrs.nested {
            display_lines.push(quote! {
                ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
                ::conflaguration::ConfigDisplay::fmt_config(&self.#field_name, __f, __depth + 1)?;
            });
            continue;
        }

        let keys = build_key_list(&struct_attrs.prefix, &field_name_str, &attrs);
        let keys_display = keys.join(", ");

        if attrs.sensitive {
            display_lines.push(quote! {
                ::std::writeln!(__f, "{}{} = *** ({})", __indent, #field_name_str, #keys_display)?;
            });
        } else {
            display_lines.push(quote! {
                ::std::writeln!(__f, "{}{} = {:?} ({})", __indent, #field_name_str, self.#field_name, #keys_display)?;
            });
        }
    }

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::conflaguration::ConfigDisplay for #struct_name #type_generics #where_clause {
            fn fmt_config(&self, __f: &mut ::std::fmt::Formatter<'_>, __depth: usize) -> ::std::fmt::Result {
                let __indent = "  ".repeat(__depth);
                #(#display_lines)*
                Ok(())
            }
        }

        impl #impl_generics ::std::fmt::Display for #struct_name #type_generics #where_clause {
            fn fmt(&self, __f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::conflaguration::ConfigDisplay::fmt_config(self, __f, 0)
            }
        }
    })
}
