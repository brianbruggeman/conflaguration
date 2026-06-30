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

/// Derive `conflaguration::Settings` to resolve a struct from environment variables.
///
/// Struct-level `#[settings(...)]`:
/// - `prefix = "APP"` — root prefix prepended to every key
/// - `resolve_with = "fn"` — default custom parser for fields without typed defaults
///
/// Field-level `#[setting(...)]`:
/// - `default` / `default = value` / `default_str = "s"` — fallback when unset
/// - `envs = "KEY"` / `envs = ["A", "B"]` — rename or cascade env keys
/// - `override` — use exact key names, ignoring the prefix
/// - `resolve_with = "fn"` — custom `fn(&str) -> Result<T, E>` parser
/// - `sensitive` — mask the value in `ConfigDisplay` output
/// - `skip` — use `Default::default()`, ignore env
///
/// Nested sub-structs (the field's type also derives `Settings`):
/// - `nested` — namespace by `{parent}_{FIELD}`
/// - `nested, prefix = "X"` — namespace by `{parent}_X`
/// - `nested, override_prefix = "X"` — absolute prefix `X`, ignoring the parent
/// - `flatten` — merge the inner fields into the parent namespace
///
/// # Example
///
/// ```rust,ignore
/// use conflaguration::{Settings, Validate, init};
///
/// #[derive(Settings, Validate)]
/// struct Database {
///     #[setting(default = "localhost")]
///     host: String,
///     #[setting(default = 5432)]
///     port: u16,
/// }
///
/// #[derive(Settings, Validate)]
/// #[settings(prefix = "APP")]
/// struct Config {
///     #[setting(default = 8080)]
///     port: u16,                         // APP_PORT
///
///     #[setting(envs = "DATABASE_URL", override)]
///     url: String,                       // DATABASE_URL  (exact key, ignores prefix)
///
///     #[setting(nested)]
///     primary: Database,                 // APP_PRIMARY_HOST, APP_PRIMARY_PORT
///
///     #[setting(nested, prefix = "RO")]
///     replica: Database,                 // APP_RO_HOST, APP_RO_PORT
///
///     #[setting(nested, override_prefix = "PG")]
///     audit: Database,                   // PG_HOST, PG_PORT  (absolute)
/// }
///
/// fn main() -> conflaguration::Result<()> {
///     let config: Config = init()?;
///     Ok(())
/// }
/// ```
#[proc_macro_derive(Settings, attributes(settings, setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    match derive_settings_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `conflaguration::Validate` to cascade validation into `nested` and
/// `flatten` fields, collecting their errors under the field name.
/// Add custom rules by implementing `Validate` manually instead.
#[proc_macro_derive(Validate, attributes(settings, setting))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    match derive_validate_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `conflaguration::ConfigDisplay` to render each field with the env key
/// it resolves from, masking `sensitive` fields and recursing into
/// `nested`/`flatten` sub-structs with their accumulated prefix.
#[proc_macro_derive(ConfigDisplay, attributes(settings, setting))]
pub fn derive_config_display(input: TokenStream) -> TokenStream {
    match derive_config_display_impl(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `conflaguration::ConfigCodegen` to emit a resolved config as build-time
/// artifacts: a `pub const` module, `cargo:rustc-cfg` directives, and the env-key
/// list for rerun tracking. Supports flat structs of scalar fields only (bool,
/// integers, floats, `String`/`&str`); `nested`/`flatten` fields are rejected and
/// `skip` fields are omitted.
#[proc_macro_derive(ConfigCodegen, attributes(settings, setting))]
pub fn derive_config_codegen(input: TokenStream) -> TokenStream {
    match derive_config_codegen_impl(input.into()) {
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
    flatten: bool,
    prefix: Option<String>,
    override_prefix: Option<String>,
    skip: bool,
    sensitive: bool,
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
        flatten: false,
        prefix: None,
        override_prefix: None,
        skip: false,
        sensitive: false,
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
                if meta.path.is_ident("flatten") {
                    attrs.flatten = true;
                    return Ok(());
                }
                if meta.path.is_ident("prefix") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    attrs.prefix = Some(lit.value());
                    return Ok(());
                }
                if meta.path.is_ident("override_prefix") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    attrs.override_prefix = Some(lit.value());
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
                Err(meta.error("unknown setting attribute"))
            })?;
        }
    }

    validate_field_attrs(field, &attrs)?;
    Ok(attrs)
}

fn validate_field_attrs(field: &syn::Field, attrs: &FieldAttrs) -> syn::Result<()> {
    let span = field.ident.as_ref().map_or_else(|| field.span(), |ident| ident.span());

    let has_any_default = attrs.default.is_some() || attrs.default_str.is_some() || attrs.use_default;
    if (attrs.default.is_some() as u8 + attrs.default_str.is_some() as u8 + attrs.use_default as u8) > 1 {
        return Err(syn::Error::new(span, "only one of default, default = value, or default_str allowed"));
    }
    if attrs.nested && attrs.flatten {
        return Err(syn::Error::new(span, "nested and flatten are mutually exclusive"));
    }
    if attrs.prefix.is_some() && attrs.override_prefix.is_some() {
        return Err(syn::Error::new(span, "only one of prefix or override_prefix allowed"));
    }
    if (attrs.prefix.is_some() || attrs.override_prefix.is_some()) && !attrs.nested {
        return Err(syn::Error::new(span, "prefix and override_prefix require nested"));
    }

    let is_sub_settings = attrs.nested || attrs.flatten;
    let has_scalar_attr = has_any_default || attrs.resolve_with.is_some() || !attrs.envs.is_empty() || attrs.envs_override || attrs.sensitive;

    if attrs.skip && (is_sub_settings || has_scalar_attr || attrs.prefix.is_some() || attrs.override_prefix.is_some()) {
        return Err(syn::Error::new(span, "skip cannot be combined with other setting attributes"));
    }
    if is_sub_settings && has_scalar_attr {
        return Err(syn::Error::new(span, "nested and flatten cannot be combined with default, default_str, resolve_with, envs, override, or sensitive"));
    }
    Ok(())
}

fn field_name_to_env_key(name: &str) -> String {
    name.to_uppercase()
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

fn field_segment_names(field_name_str: &str, attrs: &FieldAttrs) -> Vec<String> {
    if attrs.envs.is_empty() {
        vec![field_name_to_env_key(field_name_str)]
    } else {
        attrs.envs.clone()
    }
}

fn dynamic_key_tokens(field_name_str: &str, attrs: &FieldAttrs) -> (TokenStream2, TokenStream2) {
    let names = field_segment_names(field_name_str, attrs);
    let names_ref = &names;
    let keys_setup = if attrs.envs_override {
        quote! { let __keys: ::std::vec::Vec<::std::string::String> = vec![#(#names_ref.to_string()),*]; }
    } else {
        quote! { let __keys: ::std::vec::Vec<::std::string::String> = vec![#(::conflaguration::join_key(__prefix, #names_ref)),*]; }
    };
    let refs_setup = quote! { let __key_refs: ::std::vec::Vec<&str> = __keys.iter().map(|s| s.as_str()).collect(); };
    (keys_setup, refs_setup)
}

// child prefix for a nested field: absolute when override_prefix is set, else the
// accumulated parent prefix plus this field's segment (field name or explicit prefix).
fn nested_child_prefix(attrs: &FieldAttrs, field_name_str: &str) -> TokenStream2 {
    if let Some(absolute) = &attrs.override_prefix {
        return quote! { #absolute.to_string() };
    }
    let segment = attrs.prefix.clone().unwrap_or_else(|| field_name_to_env_key(field_name_str));
    quote! { ::conflaguration::join_key(__prefix, #segment) }
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

fn inherit_struct_resolve_with(attrs: &mut FieldAttrs, struct_attrs: &StructAttrs) {
    if attrs.resolve_with.is_none() && attrs.default.is_none() && !attrs.use_default {
        attrs.resolve_with.clone_from(&struct_attrs.resolve_with);
    }
}

fn gen_field_construct(field: &syn::Field, struct_attrs: &StructAttrs) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let mut attrs = parse_field_attrs(field)?;
    inherit_struct_resolve_with(&mut attrs, struct_attrs);

    if attrs.skip {
        return Ok(quote! { ::core::default::Default::default() });
    }
    let field_type = &field.ty;
    if attrs.flatten {
        return Ok(quote! { <#field_type as ::conflaguration::Settings>::from_env_with_prefix(__prefix)? });
    }
    if attrs.nested {
        let child = nested_child_prefix(&attrs, &field_name.to_string());
        return Ok(quote! { { let __child = #child; <#field_type as ::conflaguration::Settings>::from_env_with_prefix(&__child)? } });
    }
    let (keys_setup, refs_setup) = dynamic_key_tokens(&field_name.to_string(), &attrs);
    let resolve = gen_resolve_call(quote! { &__key_refs }, &attrs);
    Ok(quote! { { #keys_setup #refs_setup #resolve } })
}

fn gen_field_override(field: &syn::Field, struct_attrs: &StructAttrs) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let mut attrs = parse_field_attrs(field)?;
    inherit_struct_resolve_with(&mut attrs, struct_attrs);

    if attrs.skip {
        return Ok(quote! {});
    }
    if attrs.flatten {
        return Ok(quote! { ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, __prefix)?; });
    }
    if attrs.nested {
        let child = nested_child_prefix(&attrs, &field_name.to_string());
        return Ok(quote! { { let __child = #child; ::conflaguration::Settings::override_from_env_with_prefix(&mut self.#field_name, &__child)?; } });
    }
    let (keys_setup, refs_setup) = dynamic_key_tokens(&field_name.to_string(), &attrs);
    let guard = gen_override_guard(field_name, quote! { &__key_refs }, attrs.resolve_with.as_ref());
    Ok(quote! { { #keys_setup #refs_setup #guard } })
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

    let mut construct_exprs = Vec::new();
    let mut override_stmts = Vec::new();
    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let construct = gen_field_construct(field, &struct_attrs)?;
        let override_stmt = gen_field_override(field, &struct_attrs)?;
        construct_exprs.push(quote! { #field_name: #construct });
        override_stmts.push(override_stmt);
    }

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let prefix_const = match &struct_attrs.prefix {
        Some(pfx) => quote! { const PREFIX: ::core::option::Option<&'static str> = ::core::option::Option::Some(#pfx); },
        None => quote! { const PREFIX: ::core::option::Option<&'static str> = ::core::option::Option::None; },
    };

    Ok(quote! {
        impl #impl_generics ::conflaguration::Settings for #struct_name #type_generics #where_clause {
            #prefix_const

            fn from_env() -> ::conflaguration::Result<Self> {
                Self::from_env_with_prefix(Self::PREFIX.unwrap_or(""))
            }

            fn from_env_with_prefix(__prefix: &str) -> ::conflaguration::Result<Self> {
                Ok(Self {
                    #(#construct_exprs),*
                })
            }

            fn override_from_env(&mut self) -> ::conflaguration::Result<()> {
                self.override_from_env_with_prefix(Self::PREFIX.unwrap_or(""))
            }

            fn override_from_env_with_prefix(&mut self, __prefix: &str) -> ::conflaguration::Result<()> {
                #(#override_stmts)*
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

        if attrs.nested || attrs.flatten {
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

fn gen_display_sub_settings(field_name_str: &str, field_name: &syn::Ident, child_prefix: TokenStream2) -> TokenStream2 {
    quote! {
        ::std::writeln!(__f, "{}{}:", __indent, #field_name_str)?;
        {
            let __child = #child_prefix;
            ::conflaguration::ConfigDisplay::fmt_config_with_prefix(&self.#field_name, __f, __depth + 1, &__child)?;
        }
    }
}

fn gen_display_value(field_name_str: &str, field_name: &syn::Ident, attrs: &FieldAttrs, keys_display_expr: TokenStream2) -> TokenStream2 {
    if attrs.sensitive {
        quote! { ::std::writeln!(__f, "{}{} = *** ({})", __indent, #field_name_str, #keys_display_expr)?; }
    } else {
        quote! { ::std::writeln!(__f, "{}{} = {:?} ({})", __indent, #field_name_str, self.#field_name, #keys_display_expr)?; }
    }
}

fn gen_display_keys_expr(field_name_str: &str, attrs: &FieldAttrs) -> TokenStream2 {
    let names = field_segment_names(field_name_str, attrs);
    if attrs.envs_override {
        let joined = names.join(", ");
        return quote! { #joined };
    }
    let names_ref = &names;
    quote! {
        {
            let __keys: ::std::vec::Vec<::std::string::String> = vec![#(::conflaguration::join_key(__prefix, #names_ref)),*];
            __keys.join(", ")
        }
    }
}

fn gen_display_line(field: &syn::Field) -> syn::Result<TokenStream2> {
    let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
    let field_name_str = field_name.to_string();
    let attrs = parse_field_attrs(field)?;

    if attrs.skip {
        return Ok(gen_display_skip(&field_name_str, field_name));
    }
    if attrs.flatten {
        return Ok(gen_display_sub_settings(&field_name_str, field_name, quote! { __prefix.to_string() }));
    }
    if attrs.nested {
        let child = nested_child_prefix(&attrs, &field_name_str);
        return Ok(gen_display_sub_settings(&field_name_str, field_name, child));
    }
    let keys_expr = gen_display_keys_expr(&field_name_str, &attrs);
    Ok(gen_display_value(&field_name_str, field_name, &attrs, keys_expr))
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

    let mut lines = Vec::new();
    for field in fields {
        lines.push(gen_display_line(field)?);
    }

    let seed_prefix = match &struct_attrs.prefix {
        Some(pfx) => quote! { #pfx },
        None => quote! { "" },
    };

    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::conflaguration::ConfigDisplay for #struct_name #type_generics #where_clause {
            fn fmt_config(&self, __f: &mut ::std::fmt::Formatter<'_>, __depth: usize) -> ::std::fmt::Result {
                ::conflaguration::ConfigDisplay::fmt_config_with_prefix(self, __f, __depth, #seed_prefix)
            }

            fn fmt_config_with_prefix(&self, __f: &mut ::std::fmt::Formatter<'_>, __depth: usize, __prefix: &str) -> ::std::fmt::Result {
                let __indent = "  ".repeat(__depth);
                #(#lines)*
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

fn type_last_ident_is(field_type: &syn::Type, name: &str) -> bool {
    matches!(field_type, syn::Type::Path(path) if path.path.segments.last().is_some_and(|seg| seg.ident == name))
}

fn is_stringish(field_type: &syn::Type) -> bool {
    if type_last_ident_is(field_type, "String") {
        return true;
    }
    matches!(field_type, syn::Type::Reference(reference) if type_last_ident_is(&reference.elem, "str"))
}

fn gen_codegen_const(const_name: &str, field_name: &syn::Ident, field_type: &syn::Type) -> TokenStream2 {
    if is_stringish(field_type) {
        return quote! {
            __out.push_str("pub const ");
            __out.push_str(#const_name);
            __out.push_str(": &str = ");
            __out.push_str(&::std::format!("{:?}", self.#field_name));
            __out.push_str(";\n");
        };
    }
    let type_str = quote! { #field_type }.to_string().replace(' ', "");
    quote! {
        __out.push_str("pub const ");
        __out.push_str(#const_name);
        __out.push_str(": ");
        __out.push_str(#type_str);
        __out.push_str(" = ");
        __out.push_str(&::std::format!("{}", self.#field_name));
        __out.push_str(";\n");
    }
}

fn gen_codegen_cfg(cfg_name: &str, field_name: &syn::Ident, field_type: &syn::Type) -> TokenStream2 {
    if type_last_ident_is(field_type, "bool") {
        return quote! {
            if self.#field_name {
                ::std::println!("cargo:rustc-cfg={}_{}", __prefix, #cfg_name);
            }
        };
    }
    quote! {
        ::std::println!("cargo:rustc-cfg={}_{}={:?}", __prefix, #cfg_name, ::std::format!("{}", self.#field_name));
    }
}

fn derive_config_codegen_impl(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input: DeriveInput = syn::parse2(input)?;
    let struct_attrs = parse_struct_attrs(&input)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Err(syn::Error::new(input.ident.span(), "only named struct fields supported")),
        },
        _ => return Err(syn::Error::new(input.ident.span(), "ConfigCodegen can only be derived on structs")),
    };

    let mut const_stmts = Vec::new();
    let mut cfg_stmts = Vec::new();
    let mut env_keys: Vec<String> = Vec::new();

    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "tuple struct fields not supported"))?;
        let attrs = parse_field_attrs(field)?;

        if attrs.nested || attrs.flatten {
            return Err(syn::Error::new(field_name.span(), "ConfigCodegen does not support nested or flatten fields"));
        }
        if attrs.skip {
            continue;
        }

        let field_str = field_name.to_string();
        let const_name = field_name_to_env_key(&field_str);
        const_stmts.push(gen_codegen_const(&const_name, field_name, &field.ty));
        cfg_stmts.push(gen_codegen_cfg(&field_str, field_name, &field.ty));

        for name in field_segment_names(&field_str, &attrs) {
            let key = if attrs.envs_override {
                name
            } else {
                match &struct_attrs.prefix {
                    Some(prefix) => format!("{prefix}_{name}"),
                    None => name,
                }
            };
            if !env_keys.contains(&key) {
                env_keys.push(key);
            }
        }
    }

    let env_keys_ref = &env_keys;
    let struct_name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::conflaguration::ConfigCodegen for #struct_name #type_generics #where_clause {
            fn to_const_module(&self) -> ::std::string::String {
                let mut __out = ::std::string::String::from("// generated by conflaguration::codegen — do not edit by hand\n");
                #(#const_stmts)*
                __out
            }

            fn emit_cfg(&self, __prefix: &str) {
                #(#cfg_stmts)*
            }

            fn env_keys() -> ::std::vec::Vec<::std::string::String> {
                ::std::vec![ #( #env_keys_ref.to_string() ),* ]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

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

    #[test]
    fn nested_and_flatten_conflict_errors() {
        let input: TokenStream2 = quote! {
            struct Bad {
                #[setting(nested, flatten)]
                field: Inner,
            }
        };
        let result = derive_settings_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn prefix_without_nested_errors() {
        let input: TokenStream2 = quote! {
            struct Bad {
                #[setting(prefix = "X")]
                field: Inner,
            }
        };
        let result = derive_settings_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("require nested"));
    }

    #[test]
    fn config_codegen_rejects_enum() {
        let input: TokenStream2 = quote! { enum Bad { A } };
        let err = derive_config_codegen_impl(input).unwrap_err();
        assert!(err.to_string().contains("structs"));
    }

    #[test]
    fn config_codegen_rejects_nested() {
        let input: TokenStream2 = quote! {
            struct Bad {
                #[setting(nested)]
                inner: Inner,
            }
        };
        let err = derive_config_codegen_impl(input).unwrap_err();
        assert!(err.to_string().contains("nested or flatten"));
    }

    #[test]
    fn prefix_and_override_prefix_conflict_errors() {
        let input: TokenStream2 = quote! {
            struct Bad {
                #[setting(nested, prefix = "X", override_prefix = "Y")]
                field: Inner,
            }
        };
        let result = derive_settings_impl(input);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("only one of prefix or override_prefix"));
    }
}
