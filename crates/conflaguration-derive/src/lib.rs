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
    resolve_with: Option<syn::Path>,
}

struct FieldAttrs {
    envs: Vec<String>,
    envs_override: bool,
    default: Option<Lit>,
    default_str: Option<String>,
    use_default: bool,
    resolve_with: Option<syn::Path>,
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
    let mut resolve_with = None;
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
            if meta.path.is_ident("resolve_with") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                let path: syn::Path = lit.parse()?;
                resolve_with = Some(path);
                return Ok(());
            }
            Err(meta.error("unknown settings attribute"))
        })?;
    }
    Ok(StructAttrs { prefix, resolve_with })
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
        use_default: false,
        resolve_with: None,
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
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        attrs.default = Some(lit);
                    } else {
                        attrs.use_default = true;
                    }
                    return Ok(());
                }
                if meta.path.is_ident("default_str") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    attrs.default_str = Some(lit.value());
                    return Ok(());
                }
                if meta.path.is_ident("resolve_with") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    let path: syn::Path = lit.parse()?;
                    attrs.resolve_with = Some(path);
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

    let span = field.ident.as_ref().map_or_else(|| field.span(), |ident| ident.span());

    let has_any_default = attrs.default.is_some() || attrs.default_str.is_some() || attrs.use_default;
    if (attrs.default.is_some() as u8 + attrs.default_str.is_some() as u8 + attrs.use_default as u8) > 1 {
        return Err(syn::Error::new(span, "only one of default, default = value, or default_str allowed"));
    }
    if attrs.skip && (has_any_default || attrs.resolve_with.is_some() || !attrs.envs.is_empty() || attrs.envs_override || attrs.nested || attrs.sensitive) {
        return Err(syn::Error::new(span, "skip cannot be combined with other setting attributes"));
    }
    if attrs.nested && (has_any_default || attrs.resolve_with.is_some() || !attrs.envs.is_empty() || attrs.envs_override || attrs.sensitive) {
        return Err(syn::Error::new(span, "nested cannot be combined with default, default_str, resolve_with, envs, override, or sensitive"));
    }
    if attrs.override_prefix.is_some() && !attrs.nested {
        return Err(syn::Error::new(span, "override_prefix requires nested"));
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

fn gen_resolve_with_call(keys_expr: TokenStream2, func: &syn::Path, attrs: &FieldAttrs) -> TokenStream2 {
    if let Some(lit) = &attrs.default {
        return quote! {
            ::conflaguration::resolve_with_or(#keys_expr, #func, #lit)?
        };
    }

    if attrs.use_default {
        return quote! {
            ::conflaguration::resolve_with_or(#keys_expr, #func, ::core::default::Default::default())?
        };
    }

    if let Some(default_str) = &attrs.default_str {
        return quote! {
            ::conflaguration::resolve_with_or_str(#keys_expr, #func, #default_str)?
        };
    }

    quote! {
        ::conflaguration::resolve_with(#keys_expr, #func)?
    }
}

fn gen_resolve_call(keys_expr: TokenStream2, attrs: &FieldAttrs) -> TokenStream2 {
    if let Some(func) = &attrs.resolve_with {
        return gen_resolve_with_call(keys_expr, func, attrs);
    }

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

    if attrs.use_default {
        return quote! {
            ::conflaguration::resolve_or_else(#keys_expr, || ::core::default::Default::default())?
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

enum PrefixMode<'a> {
    Static(&'a Option<String>),
    Dynamic,
}

fn nested_prefix(field_type: &syn::Type, attrs: &FieldAttrs, prefix_mode: &PrefixMode<'_>) -> Option<TokenStream2> {
    match &attrs.override_prefix {
        Some(Some(explicit)) => Some(quote! { #explicit.to_owned() }),
        Some(None) => {
            let pfx = match prefix_mode {
                PrefixMode::Static(Some(pfx)) => quote! { #pfx },
                PrefixMode::Dynamic => quote! { __prefix },
                PrefixMode::Static(None) => return None,
            };
            Some(quote! {
                match <#field_type as ::conflaguration::Settings>::PREFIX {
                    Some(__inner) => ::std::format!("{}_{}", #pfx, __inner),
                    None => (#pfx).to_owned(),
                }
            })
        }
        None => None,
    }
}

fn gen_nested_construct(field_type: &syn::Type, prefix: Option<TokenStream2>) -> TokenStream2 {
    match prefix {
        Some(pfx) => quote! {
            { let __nested = #pfx; <#field_type as ::conflaguration::Settings>::from_env_with_prefix(&__nested)? }
        },
        None => quote! { <#field_type as ::conflaguration::Settings>::from_env()? },
    }
}

fn gen_nested_override(field_name: &syn::Ident, prefix: Option<TokenStream2>) -> TokenStream2 {
    match prefix {
        Some(pfx) => quote! {
            { let __nested = #pfx; ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, &__nested)?; }
        },
        None => quote! { ::conflaguration::Settings::override_from_env(&mut self.#field_name)?; },
    }
}

fn dynamic_key_tokens(field_name_str: &str, attrs: &FieldAttrs) -> (TokenStream2, TokenStream2) {
    let names = if attrs.envs.is_empty() { vec![field_name_to_env_key(field_name_str)] } else { attrs.envs.clone() };
    let names_ref = &names;
    let keys_setup = if attrs.envs_override {
        quote! { let __keys: Vec<String> = vec![#(#names_ref.to_string()),*]; }
    } else {
        quote! { let __keys: Vec<String> = vec![#(::std::format!("{}_{}", __prefix, #names_ref)),*]; }
    };
    let refs_setup = quote! { let __key_refs: Vec<&str> = __keys.iter().map(|s| s.as_str()).collect(); };
    (keys_setup, refs_setup)
}

fn gen_override_guard(field_name: &syn::Ident, keys_ref: TokenStream2, resolve_with: Option<&syn::Path>) -> TokenStream2 {
    let assign = match resolve_with {
        Some(func) => quote! {
            self.#field_name = ::conflaguration::resolve_with(#keys_ref, #func)?;
        },
        None => quote! {
            self.#field_name = ::conflaguration::resolve(#keys_ref)?;
        },
    };
    quote! {
        if (#keys_ref).iter().any(|__k| ::std::env::var(__k).is_ok()) {
            #assign
        }
    }
}

fn gen_construct_resolve(field_name_str: &str, attrs: &FieldAttrs, prefix_mode: &PrefixMode<'_>) -> TokenStream2 {
    match prefix_mode {
        PrefixMode::Static(prefix) => {
            let keys = build_key_list(prefix, field_name_str, attrs);
            let keys_ref = &keys;
            gen_resolve_call(quote! { &[#(#keys_ref),*] }, attrs)
        }
        PrefixMode::Dynamic => {
            let (keys_setup, refs_setup) = dynamic_key_tokens(field_name_str, attrs);
            let resolve = gen_resolve_call(quote! { &__key_refs }, attrs);
            quote! { { #keys_setup #refs_setup #resolve } }
        }
    }
}

fn gen_override_resolve(field_name: &syn::Ident, field_name_str: &str, attrs: &FieldAttrs, prefix_mode: &PrefixMode<'_>) -> TokenStream2 {
    match prefix_mode {
        PrefixMode::Static(prefix) => {
            let keys = build_key_list(prefix, field_name_str, attrs);
            let keys_ref = &keys;
            let keys_expr = quote! { &[#(#keys_ref),*] };
            let guard = gen_override_guard(field_name, quote! { __keys }, attrs.resolve_with.as_ref());
            quote! { { let __keys: &[&str] = #keys_expr; #guard } }
        }
        PrefixMode::Dynamic => {
            let (keys_setup, refs_setup) = dynamic_key_tokens(field_name_str, attrs);
            let guard = gen_override_guard(field_name, quote! { &__key_refs }, attrs.resolve_with.as_ref());
            quote! { { #keys_setup #refs_setup #guard } }
        }
    }
}

fn gen_field_construct(field: &syn::Field, prefix_mode: &PrefixMode<'_>, struct_attrs: &StructAttrs) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let mut attrs = parse_field_attrs(field)?;

    if attrs.resolve_with.is_none() && attrs.default.is_none() && !attrs.use_default {
        attrs.resolve_with.clone_from(&struct_attrs.resolve_with);
    }

    if attrs.skip {
        return Ok(quote! { ::core::default::Default::default() });
    }
    if attrs.nested {
        let prefix = nested_prefix(&field.ty, &attrs, prefix_mode);
        return Ok(gen_nested_construct(&field.ty, prefix));
    }
    Ok(gen_construct_resolve(&field_name.to_string(), &attrs, prefix_mode))
}

fn gen_field_override(field: &syn::Field, prefix_mode: &PrefixMode<'_>, struct_attrs: &StructAttrs) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let mut attrs = parse_field_attrs(field)?;

    if attrs.resolve_with.is_none() && attrs.default.is_none() && !attrs.use_default {
        attrs.resolve_with.clone_from(&struct_attrs.resolve_with);
    }

    if attrs.skip {
        return Ok(quote! {});
    }
    if attrs.nested {
        let prefix = nested_prefix(&field.ty, &attrs, prefix_mode);
        return Ok(gen_nested_override(field_name, prefix));
    }
    Ok(gen_override_resolve(field_name, &field_name.to_string(), &attrs, prefix_mode))
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

    let static_prefix = PrefixMode::Static(&struct_attrs.prefix);
    let dynamic_prefix = PrefixMode::Dynamic;

    let mut static_exprs = Vec::new();
    let mut dynamic_exprs = Vec::new();
    let mut override_static_stmts = Vec::new();
    let mut override_dynamic_stmts = Vec::new();
    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let static_expr = gen_field_construct(field, &static_prefix, &struct_attrs)?;
        let dynamic_expr = gen_field_construct(field, &dynamic_prefix, &struct_attrs)?;
        let override_static = gen_field_override(field, &static_prefix, &struct_attrs)?;
        let override_dynamic = gen_field_override(field, &dynamic_prefix, &struct_attrs)?;
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

fn gen_display_skip(field_name_str: &str, field_name: &syn::Ident) -> TokenStream2 {
    quote! { ::std::writeln!(__f, "{}{} = {:?} (skipped)", __indent, #field_name_str, self.#field_name)?; }
}

fn gen_display_nested_static(field_name_str: &str, field_name: &syn::Ident) -> TokenStream2 {
    quote! {
        ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
        ::conflaguration::ConfigDisplay::fmt_config(&self.#field_name, __f, __depth + 1)?;
    }
}

fn gen_display_nested_dynamic(field_name_str: &str, field_name: &syn::Ident, field_type: &syn::Type, attrs: &FieldAttrs) -> TokenStream2 {
    match &attrs.override_prefix {
        Some(Some(explicit)) => quote! {
            ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
            ::conflaguration::ConfigDisplay::fmt_config_with_prefix(&self.#field_name, __f, __depth + 1, #explicit)?;
        },
        Some(None) => quote! {
            ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
            {
                let __nested_pfx = match <#field_type as ::conflaguration::Settings>::PREFIX {
                    Some(__inner) => ::std::format!("{}_{}", __prefix, __inner),
                    None => __prefix.to_string(),
                };
                ::conflaguration::ConfigDisplay::fmt_config_with_prefix(&self.#field_name, __f, __depth + 1, &__nested_pfx)?;
            }
        },
        None => quote! {
            ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
            ::conflaguration::ConfigDisplay::fmt_config(&self.#field_name, __f, __depth + 1)?;
        },
    }
}

fn gen_display_value(field_name_str: &str, field_name: &syn::Ident, attrs: &FieldAttrs, keys_display_expr: TokenStream2) -> TokenStream2 {
    if attrs.sensitive {
        quote! { ::std::writeln!(__f, "{}{} = *** ({})", __indent, #field_name_str, #keys_display_expr)?; }
    } else {
        quote! { ::std::writeln!(__f, "{}{} = {:?} ({})", __indent, #field_name_str, self.#field_name, #keys_display_expr)?; }
    }
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

    let mut static_lines = Vec::new();
    let mut dynamic_lines = Vec::new();

    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let field_name_str = field_name.to_string();
        let attrs = parse_field_attrs(field)?;

        let static_keys = build_key_list(&struct_attrs.prefix, &field_name_str, &attrs);
        let static_keys_display = static_keys.join(", ");
        static_lines.push(if attrs.skip {
            gen_display_skip(&field_name_str, field_name)
        } else if attrs.nested {
            gen_display_nested_static(&field_name_str, field_name)
        } else {
            gen_display_value(&field_name_str, field_name, &attrs, quote! { #static_keys_display })
        });

        let names = if attrs.envs.is_empty() {
            vec![field_name_to_env_key(&field_name_str)]
        } else {
            attrs.envs.clone()
        };
        let names_ref = &names;
        let dynamic_keys_expr = if attrs.envs_override {
            let joined = names.join(", ");
            quote! { #joined }
        } else {
            quote! {
                {
                    let __keys: Vec<String> = vec![#(::std::format!("{}_{}", __prefix, #names_ref)),*];
                    __keys.join(", ")
                }
            }
        };
        dynamic_lines.push(if attrs.skip {
            gen_display_skip(&field_name_str, field_name)
        } else if attrs.nested {
            gen_display_nested_dynamic(&field_name_str, field_name, &field.ty, &attrs)
        } else {
            gen_display_value(&field_name_str, field_name, &attrs, dynamic_keys_expr)
        });
    }

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::conflaguration::ConfigDisplay for #struct_name #type_generics #where_clause {
            fn fmt_config(&self, __f: &mut ::std::fmt::Formatter<'_>, __depth: usize) -> ::std::fmt::Result {
                let __indent = "  ".repeat(__depth);
                #(#static_lines)*
                Ok(())
            }

            fn fmt_config_with_prefix(&self, __f: &mut ::std::fmt::Formatter<'_>, __depth: usize, __prefix: &str) -> ::std::fmt::Result {
                let __indent = "  ".repeat(__depth);
                #(#dynamic_lines)*
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_rejects_enum() {
        let input: TokenStream2 = quote! { enum Foo { A, B } };
        let result = derive_settings_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("structs"));
    }

    #[test]
    fn settings_rejects_tuple_struct() {
        let input: TokenStream2 = quote! { struct Foo(u16); };
        let result = derive_settings_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("named"));
    }

    #[test]
    fn validate_rejects_enum() {
        let input: TokenStream2 = quote! { enum Bar { X } };
        let result = derive_validate_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("structs"));
    }

    #[test]
    fn validate_rejects_tuple_struct() {
        let input: TokenStream2 = quote! { struct Bar(String); };
        let result = derive_validate_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("named"));
    }

    #[test]
    fn config_display_rejects_enum() {
        let input: TokenStream2 = quote! { enum Baz { Y } };
        let result = derive_config_display_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("structs"));
    }

    #[test]
    fn unknown_settings_attribute_errors() {
        let input: TokenStream2 = quote! {
            #[settings(bogus = "nope")]
            struct Bad {
                field: u16,
            }
        };
        let result = derive_settings_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_setting_field_attribute_errors() {
        let input: TokenStream2 = quote! {
            struct Bad {
                #[setting(bogus)]
                field: u16,
            }
        };
        let result = derive_settings_impl(input);
        assert!(result.is_err());
    }
}
