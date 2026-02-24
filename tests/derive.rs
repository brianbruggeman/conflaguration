#![cfg(feature = "derive")]
#![allow(dead_code)]

use conflaguration::ConfigDisplay;
use conflaguration::Settings;
use conflaguration::Validate;
use conflaguration::ValidationMessage;

#[derive(Settings)]
#[settings(prefix = "CONFLAG_TEST")]
struct BasicConfig {
    #[setting(default = 8080)]
    port: u16,

    #[setting(default = "localhost")]
    host: String,

    #[setting(default = false)]
    debug: bool,
}

#[test]
fn basic_defaults() {
    temp_env::with_vars([("CONFLAG_TEST_PORT", None::<&str>), ("CONFLAG_TEST_HOST", None::<&str>), ("CONFLAG_TEST_DEBUG", None::<&str>)], || {
        let config = BasicConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "localhost");
        assert!(!config.debug);
    });
}

#[test]
fn basic_reads_env() {
    temp_env::with_vars(
        [("CONFLAG_TEST_PORT", Some("3000")), ("CONFLAG_TEST_HOST", Some("0.0.0.0")), ("CONFLAG_TEST_DEBUG", Some("true"))],
        || {
            let config = BasicConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            assert_eq!(config.port, 3000);
            assert_eq!(config.host, "0.0.0.0");
            assert!(config.debug);
        },
    );
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_EXPLICIT")]
struct OverrideEnvConfig {
    #[setting(envs = "MY_PORT", r#override, default = 9090)]
    port: u16,
}

#[test]
fn override_uses_exact_key() {
    temp_env::with_vars([("CONFLAG_EXPLICIT_PORT", Some("5000")), ("MY_PORT", Some("4000"))], || {
        let config = OverrideEnvConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 4000);
    });
}

#[test]
fn override_ignores_prefixed_key() {
    temp_env::with_vars([("CONFLAG_EXPLICIT_PORT", Some("5000")), ("MY_PORT", None::<&str>)], || {
        let config = OverrideEnvConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 9090);
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_RENAME")]
struct RenamedEnvConfig {
    #[setting(envs = "TIMEOUT", default = 30)]
    timeout_secs: u64,
}

#[test]
fn envs_renames_key_with_prefix() {
    temp_env::with_vars([("CONFLAG_RENAME_TIMEOUT", Some("60")), ("CONFLAG_RENAME_TIMEOUT_SECS", Some("99"))], || {
        let config = RenamedEnvConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.timeout_secs, 60);
    });
}

#[test]
fn envs_renamed_uses_default() {
    temp_env::with_vars([("CONFLAG_RENAME_TIMEOUT", None::<&str>)], || {
        let config = RenamedEnvConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.timeout_secs, 30);
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_ENVS")]
struct MultiOverrideConfig {
    #[setting(envs = ["FIRST_PORT", "SECOND_PORT"], r#override, default = 7070)]
    port: u16,
}

#[test]
fn override_list_uses_exact_keys() {
    temp_env::with_vars([("CONFLAG_ENVS_PORT", Some("9999")), ("FIRST_PORT", Some("1111")), ("SECOND_PORT", Some("2222"))], || {
        let config = MultiOverrideConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 1111);
    });
}

#[test]
fn override_list_falls_through() {
    temp_env::with_vars([("FIRST_PORT", None::<&str>), ("SECOND_PORT", Some("6000"))], || {
        let config = MultiOverrideConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 6000);
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_MENV")]
struct MultiEnvsConfig {
    #[setting(envs = ["PORT", "HTTP_PORT"], default = 7070)]
    port: u16,
}

#[test]
fn envs_list_applies_prefix_to_each() {
    temp_env::with_vars([("CONFLAG_MENV_PORT", Some("1111")), ("CONFLAG_MENV_HTTP_PORT", Some("2222"))], || {
        let config = MultiEnvsConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 1111);
    });
}

#[test]
fn envs_list_second_prefixed_key_as_fallback() {
    temp_env::with_vars([("CONFLAG_MENV_PORT", None::<&str>), ("CONFLAG_MENV_HTTP_PORT", Some("2222"))], || {
        let config = MultiEnvsConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 2222);
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_OPT")]
struct OptionalConfig {
    workers: Option<i32>,

    #[setting(default = "app")]
    name: String,
}

#[test]
fn option_none_when_missing() {
    temp_env::with_vars([("CONFLAG_OPT_WORKERS", None::<&str>), ("CONFLAG_OPT_NAME", None::<&str>)], || {
        let config = OptionalConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.workers, None);
        assert_eq!(config.name, "app");
    });
}

#[test]
fn option_some_when_present() {
    temp_env::with_vars([("CONFLAG_OPT_WORKERS", Some("4")), ("CONFLAG_OPT_NAME", None::<&str>)], || {
        let config = OptionalConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.workers, Some(4));
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_INNER")]
struct InnerConfig {
    #[setting(default = "redis://localhost")]
    url: String,
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_OUTER")]
struct OuterConfig {
    #[setting(default = 8080)]
    port: u16,

    #[setting(nested)]
    inner: InnerConfig,
}

#[test]
fn nested_settings() {
    temp_env::with_vars([("CONFLAG_OUTER_PORT", None::<&str>), ("CONFLAG_INNER_URL", Some("redis://remote:6379"))], || {
        let config = OuterConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 8080);
        assert_eq!(config.inner.url, "redis://remote:6379");
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_SKIP")]
struct SkipConfig {
    #[setting(default = 42)]
    value: i32,

    #[setting(skip)]
    computed: String,
}

#[test]
fn skip_uses_default() {
    temp_env::with_vars([("CONFLAG_SKIP_VALUE", None::<&str>)], || {
        let config = SkipConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.value, 42);
        assert_eq!(config.computed, String::new());
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_DEFSTR")]
struct DefaultStrConfig {
    #[setting(default_str = "8080")]
    port: u16,
}

#[test]
fn default_str_parses_value() {
    temp_env::with_vars([("CONFLAG_DEFSTR_PORT", None::<&str>)], || {
        let config = DefaultStrConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 8080);
    });
}

#[derive(Settings)]
struct NoPrefixConfig {
    #[setting(default = 1234)]
    no_prefix_port: u16,
}

#[test]
fn no_prefix_uses_field_name() {
    temp_env::with_vars([("NO_PREFIX_PORT", Some("9999"))], || {
        let config = NoPrefixConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.no_prefix_port, 9999);
    });
}

#[test]
fn no_prefix_falls_back_to_default() {
    temp_env::with_vars([("NO_PREFIX_PORT", None::<&str>)], || {
        let config = NoPrefixConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.no_prefix_port, 1234);
    });
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_FLAGS")]
struct FeatureFlags {
    #[setting(default = false)]
    feature_new_ui: bool,

    #[setting(default = false)]
    feature_beta_api: bool,
}

#[test]
fn feature_flags_as_bool_fields() {
    temp_env::with_vars([("CONFLAG_FLAGS_FEATURE_NEW_UI", Some("true")), ("CONFLAG_FLAGS_FEATURE_BETA_API", None::<&str>)], || {
        let flags = FeatureFlags::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert!(flags.feature_new_ui);
        assert!(!flags.feature_beta_api);
    });
}

#[test]
fn parse_error_on_bad_value() {
    temp_env::with_vars(
        [("CONFLAG_TEST_PORT", Some("not_a_number")), ("CONFLAG_TEST_HOST", None::<&str>), ("CONFLAG_TEST_DEBUG", None::<&str>)],
        || {
            let result = BasicConfig::from_env();
            let err = match result {
                Err(err) => err,
                Ok(_) => panic!("expected parse error"),
            };
            let msg = err.to_string();
            assert!(msg.contains("CONFLAG_TEST_PORT"));
        },
    );
}

#[derive(Settings)]
#[settings(prefix = "CONFLAG_VALIDATE")]
struct ValidatedConfig {
    #[setting(default = 0)]
    port: u16,

    #[setting(default = "")]
    name: String,
}

impl conflaguration::Validate for ValidatedConfig {
    fn validate(&self) -> conflaguration::Result<()> {
        let mut errors = vec![];
        if self.port == 0 {
            errors.push(ValidationMessage::new("port", "must be > 0"));
        }
        if self.name.is_empty() {
            errors.push(ValidationMessage::new("name", "must not be empty"));
        }
        if errors.is_empty() { Ok(()) } else { Err(conflaguration::Error::Validation { errors }) }
    }
}

#[test]
fn init_validates_after_construction() {
    temp_env::with_vars([("CONFLAG_VALIDATE_PORT", Some("0")), ("CONFLAG_VALIDATE_NAME", Some(""))], || {
        let result: conflaguration::Result<ValidatedConfig> = conflaguration::init();
        let err = match result {
            Err(err) => err,
            Ok(_) => panic!("expected validation error"),
        };
        let msg = err.to_string();
        assert!(msg.contains("port: must be > 0"));
        assert!(msg.contains("name: must not be empty"));
    });
}

#[test]
fn init_passes_with_valid_config() {
    temp_env::with_vars([("CONFLAG_VALIDATE_PORT", Some("8080")), ("CONFLAG_VALIDATE_NAME", Some("myapp"))], || {
        let config: ValidatedConfig = conflaguration::init().unwrap_or_else(|err| panic!("init failed: {err}"));
        assert_eq!(config.port, 8080);
        assert_eq!(config.name, "myapp");
    });
}

#[test]
fn prefix_const_is_set() {
    assert_eq!(BasicConfig::PREFIX, Some("CONFLAG_TEST"));
    assert_eq!(NoPrefixConfig::PREFIX, None);
    assert_eq!(InnerConfig::PREFIX, Some("CONFLAG_INNER"));
}

#[test]
fn from_env_with_prefix_overrides_keys() {
    temp_env::with_vars([("CUSTOM_PORT", Some("7777")), ("CUSTOM_HOST", Some("custom.host")), ("CUSTOM_DEBUG", Some("true"))], || {
        let config = BasicConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.port, 7777);
        assert_eq!(config.host, "custom.host");
        assert!(config.debug);
    });
}

#[test]
fn from_env_with_prefix_uses_defaults() {
    temp_env::with_vars([("CUSTOM_PORT", None::<&str>), ("CUSTOM_HOST", None::<&str>), ("CUSTOM_DEBUG", None::<&str>)], || {
        let config = BasicConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "localhost");
        assert!(!config.debug);
    });
}

#[test]
fn from_env_with_prefix_override_ignores_dynamic_prefix() {
    temp_env::with_vars([("RUNTIME_PORT", Some("5555")), ("MY_PORT", Some("4444"))], || {
        let config = OverrideEnvConfig::from_env_with_prefix("RUNTIME").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.port, 4444);
    });
}

#[derive(Settings)]
#[settings(prefix = "ACCUM_INNER")]
struct AccumInner {
    #[setting(default = "default_val")]
    value: String,
}

#[derive(Settings)]
#[settings(prefix = "ACCUM_OUTER")]
struct AccumOuter {
    #[setting(default = 1)]
    count: i32,

    #[setting(nested, override_prefix)]
    inner: AccumInner,
}

#[test]
fn override_prefix_bare_accumulates() {
    temp_env::with_vars(
        [
            ("ACCUM_OUTER_COUNT", None::<&str>),
            ("ACCUM_OUTER_ACCUM_INNER_VALUE", Some("accumulated")),
            ("ACCUM_INNER_VALUE", None::<&str>),
        ],
        || {
            let config = AccumOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            assert_eq!(config.count, 1);
            assert_eq!(config.inner.value, "accumulated");
        },
    );
}

#[test]
fn override_prefix_bare_falls_back_to_default() {
    temp_env::with_vars(
        [
            ("ACCUM_OUTER_COUNT", None::<&str>),
            ("ACCUM_OUTER_ACCUM_INNER_VALUE", None::<&str>),
            ("ACCUM_INNER_VALUE", None::<&str>),
        ],
        || {
            let config = AccumOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            assert_eq!(config.inner.value, "default_val");
        },
    );
}

#[derive(Settings)]
#[settings(prefix = "EXPLICIT_OUTER")]
struct ExplicitPrefixOuter {
    #[setting(nested, override_prefix = "CUSTOM_NS")]
    inner: AccumInner,
}

#[test]
fn override_prefix_explicit_uses_given_prefix() {
    temp_env::with_vars([("CUSTOM_NS_VALUE", Some("explicit_override")), ("ACCUM_INNER_VALUE", None::<&str>)], || {
        let config = ExplicitPrefixOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.inner.value, "explicit_override");
    });
}

#[test]
fn from_env_with_prefix_propagates_to_override_prefix() {
    temp_env::with_vars([("RUNTIME_COUNT", Some("99")), ("RUNTIME_ACCUM_INNER_VALUE", Some("runtime_accumulated"))], || {
        let config = AccumOuter::from_env_with_prefix("RUNTIME").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.count, 99);
        assert_eq!(config.inner.value, "runtime_accumulated");
    });
}

#[derive(Settings, Validate)]
#[settings(prefix = "CONFLAG_NOOP")]
struct NoopValidateConfig {
    #[setting(default = false)]
    enabled: bool,

    #[setting(default = "test")]
    name: String,
}

#[test]
fn derived_validate_noop_passes() {
    let config = NoopValidateConfig::from_env_with_prefix("__UNUSED_NOOP").unwrap_or_else(|err| panic!("from_env failed: {err}"));
    assert!(config.validate().is_ok());
}

#[derive(Settings)]
#[settings(prefix = "CASCVAL_INNER")]
struct CascadeInnerConfig {
    #[setting(default = 0)]
    port: u16,
}

impl conflaguration::Validate for CascadeInnerConfig {
    fn validate(&self) -> conflaguration::Result<()> {
        let mut errors = vec![];
        if self.port == 0 {
            errors.push(ValidationMessage::new("port", "must be > 0"));
        }
        if errors.is_empty() { Ok(()) } else { Err(conflaguration::Error::Validation { errors }) }
    }
}

#[derive(Settings, Validate)]
#[settings(prefix = "CASCVAL_OUTER")]
struct CascadeOuterConfig {
    #[setting(default = false)]
    debug: bool,

    #[setting(nested)]
    inner: CascadeInnerConfig,
}

#[test]
fn derived_validate_cascades_to_nested() {
    temp_env::with_vars([("CASCVAL_OUTER_DEBUG", None::<&str>), ("CASCVAL_INNER_PORT", Some("0"))], || {
        let config = CascadeOuterConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let err = match config.validate() {
            Err(err) => err,
            Ok(()) => panic!("expected validation error"),
        };
        let msg = err.to_string();
        assert!(msg.contains("inner.port: must be > 0"));
    });
}

#[test]
fn derived_validate_cascade_passes_when_valid() {
    temp_env::with_vars([("CASCVAL_OUTER_DEBUG", None::<&str>), ("CASCVAL_INNER_PORT", Some("8080"))], || {
        let config = CascadeOuterConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert!(config.validate().is_ok());
    });
}

#[derive(Settings)]
#[settings(prefix = "CASCVAL_SECOND")]
struct CascadeSecondInner {
    #[setting(default = "")]
    name: String,
}

impl conflaguration::Validate for CascadeSecondInner {
    fn validate(&self) -> conflaguration::Result<()> {
        if self.name.is_empty() {
            Err(conflaguration::Error::Validation {
                errors: vec![ValidationMessage::new("name", "must not be empty")],
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Settings, Validate)]
#[settings(prefix = "CASCVAL_MULTI")]
struct MultiCascadeConfig {
    #[setting(nested)]
    first: CascadeInnerConfig,

    #[setting(nested)]
    second: CascadeSecondInner,
}

#[test]
fn derived_validate_collects_errors_from_multiple_nested() {
    temp_env::with_vars([("CASCVAL_INNER_PORT", Some("0")), ("CASCVAL_SECOND_NAME", Some(""))], || {
        let config = MultiCascadeConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let err = match config.validate() {
            Err(err) => err,
            Ok(()) => panic!("expected validation errors"),
        };
        let msg = err.to_string();
        assert!(msg.contains("first.port: must be > 0"));
        assert!(msg.contains("second.name: must not be empty"));
    });
}

#[derive(Settings)]
#[settings(prefix = "DEEP_L3")]
struct DeepLevel3 {
    #[setting(default = 0)]
    value: i32,
}

impl conflaguration::Validate for DeepLevel3 {
    fn validate(&self) -> conflaguration::Result<()> {
        if self.value == 0 {
            Err(conflaguration::Error::Validation {
                errors: vec![ValidationMessage::new("value", "must not be zero")],
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Settings, Validate)]
#[settings(prefix = "DEEP_L2")]
struct DeepLevel2 {
    #[setting(nested)]
    level3: DeepLevel3,
}

#[derive(Settings, Validate)]
#[settings(prefix = "DEEP_L1")]
struct DeepLevel1 {
    #[setting(nested)]
    level2: DeepLevel2,
}

#[test]
fn derived_validate_deep_nesting_builds_path() {
    temp_env::with_vars([("DEEP_L3_VALUE", Some("0"))], || {
        let config = DeepLevel1::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let err = match config.validate() {
            Err(err) => err,
            Ok(()) => panic!("expected validation error"),
        };
        let msg = err.to_string();
        assert!(msg.contains("level2.level3.value: must not be zero"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_BASIC")]
struct DisplayBasicConfig {
    #[setting(default = 8080)]
    port: u16,

    #[setting(default = "localhost")]
    host: String,

    #[setting(default = false)]
    debug: bool,
}

#[test]
fn config_display_shows_fields_and_keys() {
    temp_env::with_vars([("DISP_BASIC_PORT", None::<&str>), ("DISP_BASIC_HOST", None::<&str>), ("DISP_BASIC_DEBUG", None::<&str>)], || {
        let config = DisplayBasicConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 8080 (DISP_BASIC_PORT)"));
        assert!(output.contains("host = \"localhost\" (DISP_BASIC_HOST)"));
        assert!(output.contains("debug = false (DISP_BASIC_DEBUG)"));
    });
}

#[test]
fn config_display_shows_env_values() {
    temp_env::with_vars([("DISP_BASIC_PORT", Some("3000")), ("DISP_BASIC_HOST", Some("0.0.0.0")), ("DISP_BASIC_DEBUG", Some("true"))], || {
        let config = DisplayBasicConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 3000 (DISP_BASIC_PORT)"));
        assert!(output.contains("host = \"0.0.0.0\" (DISP_BASIC_HOST)"));
        assert!(output.contains("debug = true (DISP_BASIC_DEBUG)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_MULTI")]
struct DisplayMultiKeyConfig {
    #[setting(envs = "ALT_PORT", r#override, default = 9090)]
    port: u16,
}

#[test]
fn config_display_shows_override_key() {
    temp_env::with_vars([("ALT_PORT", None::<&str>)], || {
        let config = DisplayMultiKeyConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 9090 (ALT_PORT)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_ENVS")]
struct DisplayEnvsConfig {
    #[setting(envs = ["PORT", "HTTP_PORT"], default = 7070)]
    port: u16,
}

#[test]
fn config_display_shows_prefixed_envs_keys() {
    temp_env::with_vars([("DISP_ENVS_PORT", None::<&str>), ("DISP_ENVS_HTTP_PORT", None::<&str>)], || {
        let config = DisplayEnvsConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 7070 (DISP_ENVS_PORT, DISP_ENVS_HTTP_PORT)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_OENVS")]
struct DisplayOverrideEnvsConfig {
    #[setting(envs = ["PRIMARY_PORT", "FALLBACK_PORT"], r#override, default = 7070)]
    port: u16,
}

#[test]
fn config_display_shows_override_envs_keys() {
    temp_env::with_vars([("PRIMARY_PORT", None::<&str>), ("FALLBACK_PORT", None::<&str>)], || {
        let config = DisplayOverrideEnvsConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 7070 (PRIMARY_PORT, FALLBACK_PORT)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_INNER")]
struct DisplayInner {
    #[setting(default = "redis://localhost")]
    url: String,
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_OUTER")]
struct DisplayOuter {
    #[setting(default = 8080)]
    port: u16,

    #[setting(nested)]
    inner: DisplayInner,
}

#[test]
fn config_display_nested_indents() {
    temp_env::with_vars([("DISP_OUTER_PORT", None::<&str>), ("DISP_INNER_URL", None::<&str>)], || {
        let config = DisplayOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("port = 8080 (DISP_OUTER_PORT)"));
        assert!(output.contains("inner:"));
        assert!(output.contains("  url = \"redis://localhost\" (DISP_INNER_URL)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_SKIP")]
struct DisplaySkipConfig {
    #[setting(default = 42)]
    value: i32,

    #[setting(skip)]
    computed: String,
}

#[test]
fn config_display_shows_skipped_fields() {
    temp_env::with_vars([("DISP_SKIP_VALUE", None::<&str>)], || {
        let config = DisplaySkipConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("value = 42 (DISP_SKIP_VALUE)"));
        assert!(output.contains("computed = \"\" (skipped)"));
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_SENS")]
struct DisplaySensitiveConfig {
    #[setting(default = "admin")]
    username: String,

    #[setting(sensitive, default = "s3cret")]
    password: String,

    #[setting(envs = "API_TOKEN", r#override, sensitive)]
    token: Option<String>,
}

#[test]
fn config_display_masks_sensitive_fields() {
    temp_env::with_vars([("DISP_SENS_USERNAME", None::<&str>), ("DISP_SENS_PASSWORD", None::<&str>), ("API_TOKEN", None::<&str>)], || {
        let config = DisplaySensitiveConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("username = \"admin\" (DISP_SENS_USERNAME)"));
        assert!(output.contains("password = *** (DISP_SENS_PASSWORD)"));
        assert!(!output.contains("s3cret"));
    });
}

#[test]
fn config_display_masks_sensitive_even_when_set() {
    temp_env::with_vars(
        [("DISP_SENS_USERNAME", None::<&str>), ("DISP_SENS_PASSWORD", Some("hunter2")), ("API_TOKEN", Some("tok_abc123"))],
        || {
            let config = DisplaySensitiveConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            let output = format!("{}", config);
            assert!(output.contains("password = *** (DISP_SENS_PASSWORD)"));
            assert!(output.contains("token = *** (API_TOKEN)"));
            assert!(!output.contains("hunter2"));
            assert!(!output.contains("tok_abc123"));
        },
    );
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_OPT")]
struct DisplayOptionalConfig {
    workers: Option<i32>,

    #[setting(default = "app")]
    name: String,
}

#[test]
fn config_display_shows_option_fields() {
    temp_env::with_vars([("DISP_OPT_WORKERS", None::<&str>), ("DISP_OPT_NAME", None::<&str>)], || {
        let config = DisplayOptionalConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("workers = None (DISP_OPT_WORKERS)"));
        assert!(output.contains("name = \"app\" (DISP_OPT_NAME)"));
    });
}

#[test]
fn config_display_shows_option_some() {
    temp_env::with_vars([("DISP_OPT_WORKERS", Some("8")), ("DISP_OPT_NAME", None::<&str>)], || {
        let config = DisplayOptionalConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config);
        assert!(output.contains("workers = Some(8) (DISP_OPT_WORKERS)"));
    });
}

#[derive(Settings)]
#[settings(prefix = "OVR_BASIC")]
struct OverrideBasicConfig {
    #[setting(default = 8080)]
    port: u16,

    #[setting(default = "localhost")]
    host: String,

    #[setting(default = false)]
    debug: bool,
}

#[test]
fn override_from_env_preserves_values_when_no_env() {
    temp_env::with_vars([("OVR_BASIC_PORT", None::<&str>), ("OVR_BASIC_HOST", None::<&str>), ("OVR_BASIC_DEBUG", None::<&str>)], || {
        let mut config = OverrideBasicConfig {
            port: 3000,
            host: "filehost".into(),
            debug: true,
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "filehost");
        assert!(config.debug);
    });
}

#[test]
fn override_from_env_replaces_when_env_set() {
    temp_env::with_vars([("OVR_BASIC_PORT", Some("9090")), ("OVR_BASIC_HOST", Some("envhost")), ("OVR_BASIC_DEBUG", None::<&str>)], || {
        let mut config = OverrideBasicConfig {
            port: 3000,
            host: "filehost".into(),
            debug: true,
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "envhost");
        assert!(config.debug);
    });
}

#[derive(Settings)]
#[settings(prefix = "OVR_NEST_INNER")]
struct OverrideNestedInner {
    #[setting(default = "default_url")]
    url: String,
}

#[derive(Settings)]
#[settings(prefix = "OVR_NEST_OUTER")]
struct OverrideNestedOuter {
    #[setting(default = 8080)]
    port: u16,

    #[setting(nested)]
    inner: OverrideNestedInner,
}

#[test]
fn override_from_env_recurses_into_nested() {
    temp_env::with_vars([("OVR_NEST_OUTER_PORT", None::<&str>), ("OVR_NEST_INNER_URL", Some("redis://env:6379"))], || {
        let mut config = OverrideNestedOuter {
            port: 3000,
            inner: OverrideNestedInner { url: "redis://file:6379".into() },
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.port, 3000);
        assert_eq!(config.inner.url, "redis://env:6379");
    });
}

#[derive(Settings)]
#[settings(prefix = "OVR_OPT")]
struct OverrideOptionConfig {
    workers: Option<i32>,

    #[setting(default = "app")]
    name: String,
}

#[test]
fn override_from_env_with_option_field() {
    temp_env::with_vars([("OVR_OPT_WORKERS", Some("8")), ("OVR_OPT_NAME", None::<&str>)], || {
        let mut config = OverrideOptionConfig {
            workers: None,
            name: "from_file".into(),
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.workers, Some(8));
        assert_eq!(config.name, "from_file");
    });
}

#[test]
fn override_from_env_option_preserves_when_no_env() {
    temp_env::with_vars([("OVR_OPT_WORKERS", None::<&str>), ("OVR_OPT_NAME", None::<&str>)], || {
        let mut config = OverrideOptionConfig {
            workers: Some(4),
            name: "from_file".into(),
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.workers, Some(4));
        assert_eq!(config.name, "from_file");
    });
}

#[derive(Settings)]
#[settings(prefix = "OVR_SKIP")]
struct OverrideSkipConfig {
    #[setting(default = 42)]
    value: i32,

    #[setting(skip)]
    computed: String,
}

#[test]
fn override_from_env_skips_skip_fields() {
    temp_env::with_vars([("OVR_SKIP_VALUE", Some("99")), ("OVR_SKIP_COMPUTED", Some("should_not_apply"))], || {
        let mut config = OverrideSkipConfig {
            value: 10,
            computed: "original".into(),
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.value, 99);
        assert_eq!(config.computed, "original");
    });
}

#[derive(Settings)]
#[settings(prefix = "OVR_OVRD")]
struct OverrideWithOverrideFlag {
    #[setting(envs = "MY_SPECIAL_PORT", r#override, default = 9090)]
    port: u16,
}

#[test]
fn override_from_env_respects_override_flag() {
    temp_env::with_vars([("OVR_OVRD_PORT", Some("5555")), ("MY_SPECIAL_PORT", Some("4444"))], || {
        let mut config = OverrideWithOverrideFlag { port: 3000 };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.port, 4444);
    });
}

#[test]
fn override_from_env_with_prefix_uses_dynamic_prefix() {
    temp_env::with_vars([("RUNTIME_PORT", Some("7777")), ("RUNTIME_HOST", Some("dynamic")), ("RUNTIME_DEBUG", None::<&str>)], || {
        let mut config = OverrideBasicConfig {
            port: 3000,
            host: "filehost".into(),
            debug: true,
        };
        config
            .override_from_env_with_prefix("RUNTIME")
            .unwrap_or_else(|err| panic!("override_from_env_with_prefix failed: {err}"));
        assert_eq!(config.port, 7777);
        assert_eq!(config.host, "dynamic");
        assert!(config.debug);
    });
}

#[test]
fn config_display_with_dynamic_prefix_shows_runtime_keys() {
    temp_env::with_vars([("CUSTOM_PORT", Some("3000")), ("CUSTOM_HOST", Some("rt.host")), ("CUSTOM_DEBUG", Some("true"))], || {
        let config = DisplayBasicConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        let output = format!("{}", config.display_with_prefix("CUSTOM"));
        assert!(output.contains("port = 3000 (CUSTOM_PORT)"), "got: {output}");
        assert!(output.contains("host = \"rt.host\" (CUSTOM_HOST)"), "got: {output}");
        assert!(output.contains("debug = true (CUSTOM_DEBUG)"), "got: {output}");
    });
}

#[test]
fn config_display_with_dynamic_prefix_on_nested_without_override_prefix() {
    temp_env::with_vars([("RT_PORT", Some("5555")), ("DISP_INNER_URL", Some("redis://rt"))], || {
        let config = DisplayOuter::from_env_with_prefix("RT").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        let output = format!("{}", config.display_with_prefix("RT"));
        assert!(output.contains("port = 5555 (RT_PORT)"), "got: {output}");
        assert!(output.contains("inner:"), "got: {output}");
        assert!(output.contains("DISP_INNER_URL"), "nested without override_prefix uses static keys, got: {output}");
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DPFX_INNER")]
struct DynPrefixInner {
    #[setting(default = "default_url")]
    url: String,
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DPFX_OUTER")]
struct DynPrefixOuter {
    #[setting(default = 8080)]
    port: u16,

    #[setting(nested, override_prefix)]
    inner: DynPrefixInner,
}

#[test]
fn config_display_with_dynamic_prefix_on_override_prefix_nested() {
    temp_env::with_vars([("RT_PORT", Some("5555")), ("RT_DPFX_INNER_URL", Some("redis://rt"))], || {
        let config = DynPrefixOuter::from_env_with_prefix("RT").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        let output = format!("{}", config.display_with_prefix("RT"));
        assert!(output.contains("port = 5555 (RT_PORT)"), "got: {output}");
        assert!(output.contains("inner:"), "got: {output}");
        assert!(output.contains("RT_DPFX_INNER_URL"), "nested with override_prefix should accumulate runtime prefix, got: {output}");
    });
}

#[test]
fn from_env_with_prefix_nested_explicit_override_ignores_dynamic() {
    temp_env::with_vars(
        [("RUNTIME_INNER_VALUE", Some("should_not_use")), ("CUSTOM_NS_VALUE", Some("explicit_wins"))],
        || {
            let config =
                ExplicitPrefixOuter::from_env_with_prefix("RUNTIME").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
            assert_eq!(config.inner.value, "explicit_wins");
        },
    );
}

#[derive(Settings, Validate)]
#[settings(prefix = "NONNEST_VAL")]
struct NonNestedValidateOuter {
    #[setting(default = 1)]
    count: i32,

    #[setting(default = 0)]
    port: u16,
}

#[test]
fn non_nested_validate_does_not_cascade_to_plain_fields() {
    temp_env::with_vars([("NONNEST_VAL_COUNT", None::<&str>), ("NONNEST_VAL_PORT", Some("0"))], || {
        let config = NonNestedValidateOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert!(config.validate().is_ok());
    });
}
