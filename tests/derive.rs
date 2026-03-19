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

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_EXP_OUTER")]
struct DisplayExplicitPrefixOuter {
    #[setting(default = 8080)]
    port: u16,

    #[setting(nested, override_prefix = "DISP_FIXED_NS")]
    inner: DynPrefixInner,
}

#[test]
fn config_display_with_explicit_override_prefix_uses_fixed_prefix() {
    temp_env::with_vars([("DISP_EXP_OUTER_PORT", Some("5555")), ("DISP_FIXED_NS_URL", Some("redis://fixed"))], || {
        let config = DisplayExplicitPrefixOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{}", config.display_with_prefix("RUNTIME"));
        assert!(output.contains("port = 5555"), "got: {output}");
        assert!(output.contains("inner:"), "got: {output}");
        assert!(output.contains("DISP_FIXED_NS_URL"), "explicit override_prefix should use fixed prefix, got: {output}");
    });
}

#[test]
fn from_env_with_prefix_nested_explicit_override_ignores_dynamic() {
    temp_env::with_vars([("RUNTIME_INNER_VALUE", Some("should_not_use")), ("CUSTOM_NS_VALUE", Some("explicit_wins"))], || {
        let config = ExplicitPrefixOuter::from_env_with_prefix("RUNTIME").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.inner.value, "explicit_wins");
    });
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

fn parse_comma_list(value: &str) -> Result<Vec<String>, std::convert::Infallible> {
    Ok(value.split(',').map(|s| s.trim().to_string()).collect())
}

fn parse_key_value_pairs(value: &str) -> Result<Vec<(String, String)>, std::convert::Infallible> {
    Ok(value
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let val = parts.next()?.trim().to_string();
            Some((key, val))
        })
        .collect())
}

#[derive(Settings)]
#[settings(prefix = "FROM_FN")]
struct FromFnConfig {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,

    #[setting(default = 8080)]
    port: u16,
}

#[test]
fn resolve_with_parses_comma_list() {
    temp_env::with_vars([("FROM_FN_TAGS", Some("alpha,beta,gamma")), ("FROM_FN_PORT", None::<&str>)], || {
        let config = FromFnConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["alpha", "beta", "gamma"]);
        assert_eq!(config.port, 8080);
    });
}

#[test]
fn resolve_with_errors_when_env_not_set() {
    temp_env::with_vars([("FROM_FN_TAGS", None::<&str>), ("FROM_FN_PORT", None::<&str>)], || {
        let result = FromFnConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "FROM_FN_DEF")]
struct FromFnWithDefaultConfig {
    #[setting(resolve_with = "parse_comma_list", default_str = "x,y")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_with_default_str_uses_fn_on_fallback() {
    temp_env::with_vars([("FROM_FN_DEF_TAGS", None::<&str>)], || {
        let config = FromFnWithDefaultConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["x", "y"]);
    });
}

#[test]
fn resolve_with_with_default_str_uses_env_when_set() {
    temp_env::with_vars([("FROM_FN_DEF_TAGS", Some("a,b,c"))], || {
        let config = FromFnWithDefaultConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b", "c"]);
    });
}

#[derive(Settings)]
#[settings(prefix = "FROM_FN_KV")]
struct FromFnKeyValueConfig {
    #[setting(resolve_with = "parse_key_value_pairs")]
    pairs: Vec<(String, String)>,
}

#[test]
fn resolve_with_parses_key_value_pairs() {
    temp_env::with_vars([("FROM_FN_KV_PAIRS", Some("host=localhost,port=5432"))], || {
        let config = FromFnKeyValueConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.pairs, vec![("host".into(), "localhost".into()), ("port".into(), "5432".into())]);
    });
}

#[derive(Settings)]
#[settings(prefix = "FROM_FN_DYN")]
struct FromFnDynamicPrefixConfig {
    #[setting(resolve_with = "parse_comma_list")]
    items: Vec<String>,
}

#[test]
fn resolve_with_works_with_dynamic_prefix() {
    temp_env::with_vars([("CUSTOM_ITEMS", Some("one,two,three"))], || {
        let config = FromFnDynamicPrefixConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("from_env_with_prefix failed: {err}"));
        assert_eq!(config.items, vec!["one", "two", "three"]);
    });
}

#[derive(Settings)]
#[settings(prefix = "FROM_FN_OVR")]
struct FromFnOverrideConfig {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_override_from_env_replaces_when_set() {
    temp_env::with_vars([("FROM_FN_OVR_TAGS", Some("new,values"))], || {
        let mut config = FromFnOverrideConfig { tags: vec!["old".into()] };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.tags, vec!["new", "values"]);
    });
}

#[test]
fn resolve_with_override_from_env_preserves_when_not_set() {
    temp_env::with_vars([("FROM_FN_OVR_TAGS", None::<&str>)], || {
        let mut config = FromFnOverrideConfig { tags: vec!["keep".into()] };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override_from_env failed: {err}"));
        assert_eq!(config.tags, vec!["keep"]);
    });
}

fn strict_parse_u16(value: &str) -> Result<u16, std::num::ParseIntError> {
    value.parse()
}

#[derive(Debug, Settings)]
#[settings(prefix = "RW_FAIL")]
struct ResolveWithFailConfig {
    #[setting(resolve_with = "strict_parse_u16")]
    port: u16,
}

#[test]
fn resolve_with_parse_error_propagates() {
    temp_env::with_vars([("RW_FAIL_PORT", Some("notanumber"))], || {
        let result = ResolveWithFailConfig::from_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("RW_FAIL_PORT"), "expected key in error, got: {msg}");
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_DEF_LIT")]
struct ResolveWithDefaultLitConfig {
    #[setting(resolve_with = "strict_parse_u16", default = 3000)]
    port: u16,
}

#[test]
fn resolve_with_default_literal_uses_default_when_missing() {
    temp_env::with_vars([("RW_DEF_LIT_PORT", None::<&str>)], || {
        let config = ResolveWithDefaultLitConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 3000);
    });
}

#[test]
fn resolve_with_default_literal_uses_env_when_set() {
    temp_env::with_vars([("RW_DEF_LIT_PORT", Some("9090"))], || {
        let config = ResolveWithDefaultLitConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 9090);
    });
}

#[test]
fn resolve_with_default_literal_errors_on_bad_env() {
    temp_env::with_vars([("RW_DEF_LIT_PORT", Some("banana"))], || {
        let result = ResolveWithDefaultLitConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_DEFSTR_FAIL")]
struct ResolveWithDefaultStrFailConfig {
    #[setting(resolve_with = "strict_parse_u16", default_str = "banana")]
    port: u16,
}

#[test]
fn resolve_with_default_str_errors_when_default_str_unparseable() {
    temp_env::with_vars([("RW_DEFSTR_FAIL_PORT", None::<&str>)], || {
        let result = ResolveWithDefaultStrFailConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_CASCADE")]
struct ResolveWithCascadeConfig {
    #[setting(envs = ["PORT", "HTTP_PORT"], resolve_with = "strict_parse_u16")]
    port: u16,
}

#[test]
fn resolve_with_cascades_through_envs_list() {
    temp_env::with_vars([("RW_CASCADE_PORT", None::<&str>), ("RW_CASCADE_HTTP_PORT", Some("7777"))], || {
        let config = ResolveWithCascadeConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 7777);
    });
}

#[test]
fn resolve_with_cascade_uses_first_match() {
    temp_env::with_vars([("RW_CASCADE_PORT", Some("1111")), ("RW_CASCADE_HTTP_PORT", Some("2222"))], || {
        let config = ResolveWithCascadeConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 1111);
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_OVR_FAIL")]
struct ResolveWithOverrideFailConfig {
    #[setting(resolve_with = "strict_parse_u16")]
    port: u16,
}

#[test]
fn resolve_with_override_from_env_errors_on_bad_value() {
    temp_env::with_vars([("RW_OVR_FAIL_PORT", Some("notanumber"))], || {
        let mut config = ResolveWithOverrideFailConfig { port: 8080 };
        let result = config.override_from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_DYN_FAIL")]
struct ResolveWithDynamicPrefixFailConfig {
    #[setting(resolve_with = "strict_parse_u16")]
    port: u16,
}

#[test]
fn resolve_with_dynamic_prefix_errors_on_bad_value() {
    temp_env::with_vars([("CUSTOM_PORT", Some("notanumber"))], || {
        let result = ResolveWithDynamicPrefixFailConfig::from_env_with_prefix("CUSTOM");
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_FAIL", resolve_with = "strict_parse_u16")]
struct StructLevelResolveWithFailConfig {
    port: u16,
}

#[test]
fn struct_level_resolve_with_errors_on_bad_value() {
    temp_env::with_vars([("STRUCT_RW_FAIL_PORT", Some("notanumber"))], || {
        let result = StructLevelResolveWithFailConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW", resolve_with = "parse_comma_list")]
struct StructLevelResolveWithConfig {
    tags: Vec<String>,

    labels: Vec<String>,
}

#[test]
fn struct_level_resolve_with_applies_to_all_fields() {
    temp_env::with_vars([("STRUCT_RW_TAGS", Some("a,b")), ("STRUCT_RW_LABELS", Some("x,y,z"))], || {
        let config = StructLevelResolveWithConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.labels, vec!["x", "y", "z"]);
    });
}

fn parse_upper(value: &str) -> Result<Vec<String>, std::convert::Infallible> {
    Ok(value.split(',').map(|s| s.trim().to_uppercase()).collect())
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_MIX", resolve_with = "parse_comma_list")]
struct StructLevelWithFieldOverride {
    tags: Vec<String>,

    #[setting(resolve_with = "parse_upper")]
    labels: Vec<String>,
}

#[test]
fn field_level_resolve_with_overrides_struct_level() {
    temp_env::with_vars([("STRUCT_RW_MIX_TAGS", Some("a,b")), ("STRUCT_RW_MIX_LABELS", Some("x,y"))], || {
        let config = StructLevelWithFieldOverride::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.labels, vec!["X", "Y"]);
    });
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_SKIP", resolve_with = "parse_comma_list")]
struct StructLevelSkipsNestedAndSkip {
    tags: Vec<String>,

    #[setting(skip)]
    computed: String,
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_TYPED", resolve_with = "parse_comma_list")]
struct StructLevelWithTypedDefault {
    tags: Vec<String>,

    #[setting(default = 8080)]
    port: u16,

    #[setting(default)]
    debug: bool,
}

#[test]
fn struct_level_resolve_with_skips_fields_with_typed_default() {
    temp_env::with_vars([("STRUCT_RW_TYPED_TAGS", Some("a,b")), ("STRUCT_RW_TYPED_PORT", None::<&str>), ("STRUCT_RW_TYPED_DEBUG", None::<&str>)], || {
        let config = StructLevelWithTypedDefault::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.port, 8080);
        assert!(!config.debug);
    });
}

#[test]
fn struct_level_resolve_with_typed_default_field_reads_env() {
    temp_env::with_vars([("STRUCT_RW_TYPED_TAGS", Some("x")), ("STRUCT_RW_TYPED_PORT", Some("3000")), ("STRUCT_RW_TYPED_DEBUG", Some("true"))], || {
        let config = StructLevelWithTypedDefault::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["x"]);
        assert_eq!(config.port, 3000);
        assert!(config.debug);
    });
}

#[test]
fn struct_level_resolve_with_skips_skip_fields() {
    temp_env::with_vars([("STRUCT_RW_SKIP_TAGS", Some("a,b"))], || {
        let config = StructLevelSkipsNestedAndSkip::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.computed, String::new());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_ENVS_OVR")]
struct ResolveWithEnvsOverrideConfig {
    #[setting(envs = "CUSTOM_TAGS", r#override, resolve_with = "parse_comma_list")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_envs_override_uses_exact_key() {
    temp_env::with_vars([("CUSTOM_TAGS", Some("a,b")), ("RW_ENVS_OVR_TAGS", Some("x,y"))], || {
        let config = ResolveWithEnvsOverrideConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
    });
}

#[test]
fn resolve_with_envs_override_errors_on_missing() {
    temp_env::with_vars([("CUSTOM_TAGS", None::<&str>), ("RW_ENVS_OVR_TAGS", Some("ignored"))], || {
        let result = ResolveWithEnvsOverrideConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_DEFSTR_ENV_FAIL")]
struct ResolveWithDefaultStrEnvFailConfig {
    #[setting(resolve_with = "strict_parse_u16", default_str = "8080")]
    port: u16,
}

#[test]
fn resolve_with_default_str_env_parse_error_does_not_fall_to_default() {
    temp_env::with_vars([("RW_DEFSTR_ENV_FAIL_PORT", Some("banana"))], || {
        let result = ResolveWithDefaultStrEnvFailConfig::from_env();
        assert!(result.is_err());
    });
}

#[test]
fn resolve_with_default_str_env_parse_happy() {
    temp_env::with_vars([("RW_DEFSTR_ENV_FAIL_PORT", Some("9090"))], || {
        let config = ResolveWithDefaultStrEnvFailConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 9090);
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_DYN_DEF")]
struct ResolveWithDynamicPrefixDefaultConfig {
    #[setting(resolve_with = "parse_comma_list", default_str = "fallback")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_dynamic_prefix_uses_default_when_missing() {
    temp_env::with_vars([("CUSTOM_TAGS", None::<&str>)], || {
        let config = ResolveWithDynamicPrefixDefaultConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.tags, vec!["fallback"]);
    });
}

#[test]
fn resolve_with_dynamic_prefix_uses_env_when_set() {
    temp_env::with_vars([("CUSTOM_TAGS", Some("a,b"))], || {
        let config = ResolveWithDynamicPrefixDefaultConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_OVR_PFX")]
struct ResolveWithOverridePrefixConfig {
    #[setting(resolve_with = "strict_parse_u16")]
    port: u16,
}

#[test]
fn resolve_with_override_from_env_with_prefix_replaces() {
    temp_env::with_vars([("RUNTIME_PORT", Some("7777"))], || {
        let mut config = ResolveWithOverridePrefixConfig { port: 3000 };
        config
            .override_from_env_with_prefix("RUNTIME")
            .unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.port, 7777);
    });
}

#[test]
fn resolve_with_override_from_env_with_prefix_preserves_when_missing() {
    temp_env::with_vars([("RUNTIME_PORT", None::<&str>)], || {
        let mut config = ResolveWithOverridePrefixConfig { port: 3000 };
        config
            .override_from_env_with_prefix("RUNTIME")
            .unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.port, 3000);
    });
}

#[test]
fn resolve_with_override_from_env_with_prefix_errors_on_bad_value() {
    temp_env::with_vars([("RUNTIME_PORT", Some("banana"))], || {
        let mut config = ResolveWithOverridePrefixConfig { port: 3000 };
        let result = config.override_from_env_with_prefix("RUNTIME");
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_NEST_INNER")]
struct StructRwNestInner {
    #[setting(default = "inner_val")]
    value: String,
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_NEST", resolve_with = "parse_comma_list")]
struct StructLevelResolveWithNestedConfig {
    tags: Vec<String>,

    #[setting(nested)]
    inner: StructRwNestInner,
}

#[test]
fn struct_level_resolve_with_ignores_nested_fields() {
    temp_env::with_vars([("STRUCT_RW_NEST_TAGS", Some("a,b")), ("STRUCT_RW_NEST_INNER_VALUE", Some("hello"))], || {
        let config = StructLevelResolveWithNestedConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.inner.value, "hello");
    });
}

#[derive(Settings)]
#[settings(prefix = "STRUCT_RW_MISS")]
struct StructLevelResolveWithMissingConfig {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,
}

#[test]
fn struct_level_resolve_with_errors_when_env_missing() {
    temp_env::with_vars([("STRUCT_RW_MISS_TAGS", None::<&str>)], || {
        let result = StructLevelResolveWithMissingConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_RW")]
struct DisplayResolveWithConfig {
    #[setting(resolve_with = "parse_comma_list", default_str = "x")]
    tags: Vec<String>,

    #[setting(default = 8080)]
    port: u16,
}

#[test]
fn config_display_shows_resolve_with_field_value() {
    temp_env::with_vars([("DISP_RW_TAGS", Some("a,b,c")), ("DISP_RW_PORT", None::<&str>)], || {
        let config = DisplayResolveWithConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{config}");
        assert!(output.contains("tags = [\"a\", \"b\", \"c\"] (DISP_RW_TAGS)"), "got: {output}");
        assert!(output.contains("port = 8080 (DISP_RW_PORT)"), "got: {output}");
    });
}

#[test]
fn config_display_with_prefix_shows_resolve_with_field() {
    temp_env::with_vars([("CUSTOM_TAGS", Some("x,y")), ("CUSTOM_PORT", None::<&str>)], || {
        let config = DisplayResolveWithConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("failed: {err}"));
        let output = format!("{}", config.display_with_prefix("CUSTOM"));
        assert!(output.contains("tags = [\"x\", \"y\"] (CUSTOM_TAGS)"), "got: {output}");
    });
}

#[derive(Default, Debug, Settings)]
#[settings(prefix = "BUILD_RW")]
struct BuilderResolveWithConfig {
    #[setting(resolve_with = "parse_comma_list", default_str = "default")]
    tags: Vec<String>,

    #[setting(default = 3000)]
    port: u16,
}

impl conflaguration::Validate for BuilderResolveWithConfig {
    fn validate(&self) -> conflaguration::Result<()> {
        if self.tags.is_empty() {
            return Err(conflaguration::Error::Validation {
                errors: vec![ValidationMessage::new("tags", "must not be empty")],
            });
        }
        Ok(())
    }
}

#[test]
fn builder_with_resolve_with_env_happy() {
    temp_env::with_vars([("BUILD_RW_TAGS", Some("a,b")), ("BUILD_RW_PORT", Some("9090"))], || {
        let config: BuilderResolveWithConfig = conflaguration::builder()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.port, 9090);
    });
}

#[test]
fn builder_with_resolve_with_defaults_then_env() {
    temp_env::with_vars([("BUILD_RW_TAGS", Some("x,y")), ("BUILD_RW_PORT", None::<&str>)], || {
        let config: BuilderResolveWithConfig = conflaguration::builder()
            .defaults()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.tags, vec!["x", "y"]);
        assert_eq!(config.port, 0);
    });
}

#[derive(Debug, Settings, ConfigDisplay)]
#[settings(prefix = "DISP_RW_SENS")]
struct DisplayResolveWithSensitiveConfig {
    #[setting(resolve_with = "parse_comma_list", sensitive, default_str = "secret")]
    tokens: Vec<String>,

    #[setting(default = "visible")]
    name: String,
}

#[test]
fn config_display_masks_resolve_with_sensitive_field() {
    temp_env::with_vars([("DISP_RW_SENS_TOKENS", Some("tok1,tok2")), ("DISP_RW_SENS_NAME", None::<&str>)], || {
        let config = DisplayResolveWithSensitiveConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        let output = format!("{config}");
        assert!(output.contains("tokens = *** (DISP_RW_SENS_TOKENS)"), "got: {output}");
        assert!(!output.contains("tok1"), "sensitive value leaked: {output}");
        assert!(output.contains("name = \"visible\" (DISP_RW_SENS_NAME)"), "got: {output}");
    });
}

#[test]
fn config_display_masks_resolve_with_sensitive_even_with_dynamic_prefix() {
    temp_env::with_vars([("CUSTOM_TOKENS", Some("secret_tok")), ("CUSTOM_NAME", None::<&str>)], || {
        let config = DisplayResolveWithSensitiveConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("failed: {err}"));
        let output = format!("{}", config.display_with_prefix("CUSTOM"));
        assert!(output.contains("tokens = ***"), "got: {output}");
        assert!(!output.contains("secret_tok"), "sensitive value leaked: {output}");
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_EMPTY")]
struct ResolveWithEmptyEnvConfig {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_receives_empty_string_when_env_is_empty() {
    temp_env::with_vars([("RW_EMPTY_TAGS", Some(""))], || {
        let config = ResolveWithEmptyEnvConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec![""]);
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_EMPTY_DEF")]
struct ResolveWithEmptyEnvDefaultConfig {
    #[setting(resolve_with = "parse_comma_list", default_str = "fallback")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_empty_env_does_not_fall_to_default() {
    temp_env::with_vars([("RW_EMPTY_DEF_TAGS", Some(""))], || {
        let config = ResolveWithEmptyEnvDefaultConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec![""], "empty string should be parsed, not treated as missing");
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_MULTI_OVR")]
struct ResolveWithMultiEnvsOverrideConfig {
    #[setting(envs = ["PRIMARY", "FALLBACK"], r#override, resolve_with = "parse_comma_list")]
    tags: Vec<String>,
}

#[test]
fn resolve_with_multi_envs_override_uses_first() {
    temp_env::with_vars([("PRIMARY", Some("a,b")), ("FALLBACK", Some("x,y"))], || {
        let config = ResolveWithMultiEnvsOverrideConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
    });
}

#[test]
fn resolve_with_multi_envs_override_falls_to_second() {
    temp_env::with_vars([("PRIMARY", None::<&str>), ("FALLBACK", Some("x,y"))], || {
        let config = ResolveWithMultiEnvsOverrideConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["x", "y"]);
    });
}

#[test]
fn resolve_with_multi_envs_override_errors_when_all_missing() {
    temp_env::with_vars([("PRIMARY", None::<&str>), ("FALLBACK", None::<&str>)], || {
        let result = ResolveWithMultiEnvsOverrideConfig::from_env();
        assert!(result.is_err());
    });
}

#[derive(Settings)]
#[settings(prefix = "RW_COEXIST_INNER")]
struct ResolveWithCoexistInner {
    #[setting(default = "inner_default")]
    value: String,
}

#[derive(Settings)]
#[settings(prefix = "RW_COEXIST")]
struct ResolveWithCoexistConfig {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,

    #[setting(nested)]
    inner: ResolveWithCoexistInner,

    #[setting(default = 8080)]
    port: u16,
}

#[test]
fn field_resolve_with_coexists_with_nested_and_regular_fields() {
    temp_env::with_vars([("RW_COEXIST_TAGS", Some("a,b")), ("RW_COEXIST_INNER_VALUE", Some("hello")), ("RW_COEXIST_PORT", Some("9090"))], || {
        let config = ResolveWithCoexistConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
        assert_eq!(config.inner.value, "hello");
        assert_eq!(config.port, 9090);
    });
}

#[test]
fn field_resolve_with_coexists_nested_uses_defaults() {
    temp_env::with_vars([("RW_COEXIST_TAGS", Some("x")), ("RW_COEXIST_INNER_VALUE", None::<&str>), ("RW_COEXIST_PORT", None::<&str>)], || {
        let config = ResolveWithCoexistConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["x"]);
        assert_eq!(config.inner.value, "inner_default");
        assert_eq!(config.port, 8080);
    });
}

#[test]
fn field_resolve_with_error_does_not_affect_nested() {
    temp_env::with_vars([("RW_COEXIST_TAGS", None::<&str>), ("RW_COEXIST_INNER_VALUE", Some("fine")), ("RW_COEXIST_PORT", Some("3000"))], || {
        let result = ResolveWithCoexistConfig::from_env();
        assert!(result.is_err(), "resolve_with field missing should error even though siblings are fine");
    });
}

#[derive(Debug, Clone, PartialEq)]
struct ColonRecord {
    left: String,
    right: String,
}

#[derive(Debug)]
struct ParseError(String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::error::Error for ParseError {}

fn parse_colon_pair(value: &str) -> Result<ColonRecord, ParseError> {
    let (left, right) = value
        .split_once(':')
        .ok_or_else(|| ParseError(format!("expected colon-delimited pair: {value}")))?;
    Ok(ColonRecord {
        left: left.to_string(),
        right: right.to_string(),
    })
}

#[derive(Debug, Settings)]
#[settings(prefix = "COLON")]
struct ColonConfig {
    #[setting(resolve_with = "parse_colon_pair")]
    pair: ColonRecord,

    #[setting(default = 8080)]
    port: u16,
}

#[test]
fn resolve_with_custom_struct_parses_correctly() {
    temp_env::with_vars([("COLON_PAIR", Some("host:5432")), ("COLON_PORT", None::<&str>)], || {
        let config = ColonConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.pair.left, "host");
        assert_eq!(config.pair.right, "5432");
        assert_eq!(config.port, 8080);
    });
}

#[test]
fn resolve_with_custom_struct_rejects_malformed() {
    temp_env::with_vars([("COLON_PAIR", Some("no-colon-here")), ("COLON_PORT", None::<&str>)], || {
        let result = ColonConfig::from_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("COLON_PAIR"), "error should name the key, got: {msg}");
    });
}

#[test]
fn resolve_with_custom_struct_missing_env_errors() {
    temp_env::with_vars([("COLON_PAIR", None::<&str>), ("COLON_PORT", None::<&str>)], || {
        let result = ColonConfig::from_env();
        assert!(result.is_err());
    });
}

#[test]
fn resolve_with_custom_struct_dynamic_prefix() {
    temp_env::with_vars([("STAGING_PAIR", Some("db:3306")), ("STAGING_PORT", Some("3000"))], || {
        let config = ColonConfig::from_env_with_prefix("STAGING").unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.pair.left, "db");
        assert_eq!(config.pair.right, "3306");
        assert_eq!(config.port, 3000);
    });
}

#[test]
fn resolve_with_custom_struct_override_replaces() {
    temp_env::with_vars([("COLON_PAIR", Some("new:9999"))], || {
        let mut config = ColonConfig {
            pair: ColonRecord {
                left: "old".into(),
                right: "0".into(),
            },
            port: 8080,
        };
        config
            .override_from_env()
            .unwrap_or_else(|err| panic!("override failed: {err}"));
        assert_eq!(config.pair.left, "new");
        assert_eq!(config.pair.right, "9999");
    });
}

#[test]
fn builder_with_resolve_with_validate_sad() {
    temp_env::with_vars([("BUILD_RW_TAGS", None::<&str>), ("BUILD_RW_PORT", None::<&str>)], || {
        let result: conflaguration::Result<BuilderResolveWithConfig> = conflaguration::builder().defaults().validate().build();
        assert!(matches!(result, Err(conflaguration::Error::Validation { .. })));
    });
}

#[derive(Settings)]
#[settings(prefix = "BARE_DEF")]
struct BareDefaultConfig {
    #[setting(default)]
    port: u16,

    #[setting(default)]
    debug: bool,

    #[setting(default)]
    name: String,
}

#[test]
fn bare_default_uses_type_default_when_missing() {
    temp_env::with_vars([("BARE_DEF_PORT", None::<&str>), ("BARE_DEF_DEBUG", None::<&str>), ("BARE_DEF_NAME", None::<&str>)], || {
        let config = BareDefaultConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 0);
        assert!(!config.debug);
        assert_eq!(config.name, "");
    });
}

#[test]
fn bare_default_uses_env_when_set() {
    temp_env::with_vars([("BARE_DEF_PORT", Some("9090")), ("BARE_DEF_DEBUG", Some("true")), ("BARE_DEF_NAME", Some("myapp"))], || {
        let config = BareDefaultConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 9090);
        assert!(config.debug);
        assert_eq!(config.name, "myapp");
    });
}

#[test]
fn bare_default_with_dynamic_prefix() {
    temp_env::with_vars([("CUSTOM_PORT", None::<&str>), ("CUSTOM_DEBUG", Some("true")), ("CUSTOM_NAME", None::<&str>)], || {
        let config = BareDefaultConfig::from_env_with_prefix("CUSTOM").unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.port, 0);
        assert!(config.debug);
        assert_eq!(config.name, "");
    });
}

#[derive(Settings)]
#[settings(prefix = "BARE_DEF_OVR")]
struct BareDefaultOverrideConfig {
    #[setting(default)]
    port: u16,

    #[setting(default)]
    name: String,
}

#[test]
fn bare_default_override_preserves_when_env_missing() {
    temp_env::with_vars([("BARE_DEF_OVR_PORT", None::<&str>), ("BARE_DEF_OVR_NAME", None::<&str>)], || {
        let mut config = BareDefaultOverrideConfig { port: 9090, name: "kept".into() };
        config.override_from_env().unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.name, "kept");
    });
}

#[test]
fn bare_default_override_replaces_when_env_set() {
    temp_env::with_vars([("BARE_DEF_OVR_PORT", Some("3000")), ("BARE_DEF_OVR_NAME", Some("replaced"))], || {
        let mut config = BareDefaultOverrideConfig { port: 9090, name: "kept".into() };
        config.override_from_env().unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.port, 3000);
        assert_eq!(config.name, "replaced");
    });
}

#[derive(Settings)]
#[settings(prefix = "BARE_DEF_RW")]
struct BareDefaultResolveWithConfig {
    #[setting(resolve_with = "parse_comma_list", default)]
    tags: Vec<String>,
}

#[test]
fn bare_default_with_resolve_with_uses_empty_vec_when_missing() {
    temp_env::with_vars([("BARE_DEF_RW_TAGS", None::<&str>)], || {
        let config = BareDefaultResolveWithConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert!(config.tags.is_empty());
    });
}

#[test]
fn bare_default_with_resolve_with_uses_env_when_set() {
    temp_env::with_vars([("BARE_DEF_RW_TAGS", Some("a,b"))], || {
        let config = BareDefaultResolveWithConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.tags, vec!["a", "b"]);
    });
}

#[derive(Settings)]
#[settings(prefix = "OPT_PARSE")]
struct OptionParseConfig {
    port: Option<u16>,
}

#[test]
fn option_present_invalid_returns_parse_error() {
    temp_env::with_vars([("OPT_PARSE_PORT", Some("banana"))], || {
        let result = OptionParseConfig::from_env();
        assert!(result.is_err(), "present but unparseable Option<u16> should error");
    });
}

#[test]
fn option_missing_returns_none() {
    temp_env::with_vars([("OPT_PARSE_PORT", None::<&str>)], || {
        let config = OptionParseConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, None);
    });
}

#[test]
fn option_present_valid_returns_some() {
    temp_env::with_vars([("OPT_PARSE_PORT", Some("8080"))], || {
        let config = OptionParseConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, Some(8080));
    });
}

#[derive(Settings)]
#[settings(prefix = "DUP_ENV")]
struct DuplicateEnvAliasConfig {
    #[setting(envs = ["PORT", "PORT"], default = 3000)]
    port: u16,
}

#[test]
fn duplicate_env_aliases_deduped_uses_value() {
    temp_env::with_vars([("DUP_ENV_PORT", Some("9090"))], || {
        let config = DuplicateEnvAliasConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 9090);
    });
}

#[test]
fn duplicate_env_aliases_deduped_uses_default() {
    temp_env::with_vars([("DUP_ENV_PORT", None::<&str>)], || {
        let config = DuplicateEnvAliasConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.port, 3000);
    });
}

#[derive(Debug, Settings)]
#[settings(prefix = "NEST_ERR_INNER")]
struct NestedErrorInner {
    port: u16,
}

#[derive(Debug, Settings)]
#[settings(prefix = "NEST_ERR_OUTER")]
struct NestedErrorOuter {
    #[setting(default = "ok")]
    name: String,

    #[setting(nested)]
    inner: NestedErrorInner,
}

#[test]
fn nested_parse_error_surfaces_while_sibling_is_valid() {
    temp_env::with_vars([("NEST_ERR_OUTER_NAME", Some("fine")), ("NEST_ERR_INNER_PORT", Some("banana"))], || {
        let result = NestedErrorOuter::from_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("NEST_ERR_INNER_PORT"), "error should identify the nested field's key, got: {msg}");
    });
}

#[test]
fn nested_missing_required_field_errors() {
    temp_env::with_vars([("NEST_ERR_OUTER_NAME", Some("fine")), ("NEST_ERR_INNER_PORT", None::<&str>)], || {
        let result = NestedErrorOuter::from_env();
        assert!(result.is_err());
    });
}

#[test]
fn nested_all_valid_succeeds() {
    temp_env::with_vars([("NEST_ERR_OUTER_NAME", Some("hello")), ("NEST_ERR_INNER_PORT", Some("5432"))], || {
        let config = NestedErrorOuter::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
        assert_eq!(config.name, "hello");
        assert_eq!(config.inner.port, 5432);
    });
}
