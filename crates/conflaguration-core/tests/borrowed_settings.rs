use std::ffi::{OsStr, OsString};
use std::num::ParseIntError;

use conflaguration::{Settings, Validate};
use conflaguration_core::{BorrowedSettings, ResolveError, ResolveFromSource, SettingState, SettingsLayers, SettingsSource, resolve_color};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "alloc")]
use conflaguration::Validate as ExistingValidate;
#[cfg(feature = "alloc")]
use conflaguration_core::{Validate as BorrowedValidate, ValidationError};

#[test]
fn no_color_presence() {
    let default_pairs: [(&str, &str); 0] = [];
    let default_source = BorrowedSettings::new(&default_pairs);
    let generated_default = resolve_color(&default_source, true).expect("generated color default resolves");

    let disabled_pairs = [("CARGO_VAL_COLOR", "false")];
    let disabled_source = BorrowedSettings::new(&disabled_pairs);
    let configured_false = resolve_color(&disabled_source, true).expect("boolean color override resolves");

    let enabled_pairs = [("CARGO_VAL_COLOR", "true")];
    let enabled_source = BorrowedSettings::new(&enabled_pairs);
    let absent_no_color = resolve_color(&enabled_source, false).expect("absent NO_COLOR preserves setting");

    let no_color_pairs = [("CARGO_VAL_COLOR", "true"), ("NO_COLOR", "")];
    let no_color_source = BorrowedSettings::new(&no_color_pairs);
    let present_empty_no_color = resolve_color(&no_color_source, false).expect("present empty NO_COLOR disables color");
    let mutation_rejected = present_empty_no_color != absent_no_color;

    assert!(generated_default);
    assert!(!configured_false);
    assert!(absent_no_color);
    assert!(!present_empty_no_color);
    assert!(mutation_rejected);

    println!(
        "{{\"generated_default_true\":{},\"color_override_false\":{},\"absent_no_color_preserves\":{},\"present_empty_no_color_disables\":{},\"owned_pair_copies\":0,\"mutation_rejected\":{}}}",
        usize::from(generated_default),
        usize::from(!configured_false),
        usize::from(absent_no_color),
        usize::from(!present_empty_no_color),
        usize::from(mutation_rejected)
    );
}

struct LayeredCargoSettings {
    jobs: usize,
    color: bool,
}

static VALIDATION_CALLS: AtomicUsize = AtomicUsize::new(0);

impl conflaguration::Validate for LayeredCargoSettings {
    fn validate(&self) -> conflaguration::Result<()> {
        VALIDATION_CALLS.fetch_add(1, Ordering::Relaxed);
        if self.jobs == 0 {
            Err(conflaguration::Error::Validation {
                errors: vec![conflaguration::ValidationMessage::new("jobs", "must be > 0")],
            })
        } else {
            Ok(())
        }
    }
}

#[test]
fn cli_precedence() {
    let default_pairs = [("JOBS", "2"), ("COLOR", "true")];
    let environment_pairs = [("JOBS", "3"), ("COLOR", "false")];
    let cli_pairs = [("JOBS", "4"), ("COLOR", "true")];
    let defaults = BorrowedSettings::new(&default_pairs);
    let environment = BorrowedSettings::new(&environment_pairs);
    let cli = BorrowedSettings::new(&cli_pairs);
    let layers = SettingsLayers::new(&defaults, &environment, &cli);
    let resolved = LayeredCargoSettings {
        jobs: layers.parse_scalar::<usize>("JOBS").expect("JOBS parses").expect("JOBS exists"),
        color: layers
            .parse_scalar::<bool>("COLOR")
            .expect("COLOR parses")
            .expect("COLOR exists"),
    };
    let previous_calls = VALIDATION_CALLS.load(Ordering::Relaxed);
    resolved.validate().expect("positive CLI jobs validate");
    let validation_calls = VALIDATION_CALLS.load(Ordering::Relaxed) - previous_calls;

    let invalid_cli_pairs = [("JOBS", "0"), ("COLOR", "true")];
    let invalid_cli = BorrowedSettings::new(&invalid_cli_pairs);
    let invalid_layers = SettingsLayers::new(&defaults, &environment, &invalid_cli);
    let invalid = LayeredCargoSettings {
        jobs: invalid_layers
            .parse_scalar::<usize>("JOBS")
            .expect("JOBS parses")
            .expect("JOBS exists"),
        color: invalid_layers
            .parse_scalar::<bool>("COLOR")
            .expect("COLOR parses")
            .expect("COLOR exists"),
    };
    let invalid_rejected = invalid.validate().is_err();

    assert_eq!(resolved.jobs, 4);
    assert!(resolved.color);
    assert_eq!(validation_calls, 1);
    assert!(invalid_rejected);

    println!(
        "{{\"generated_jobs\":2,\"environment_jobs\":3,\"cli_jobs\":4,\"resolved_jobs\":{},\"validation_calls\":{validation_calls},\"cli_jobs_zero_rejected\":{}}}",
        resolved.jobs,
        usize::from(invalid_rejected)
    );
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SerializableSettings {
    jobs: usize,
    color: bool,
}

#[test]
fn builder_roundtrip() {
    let built = conflaguration::ConfigBuilder::new()
        .value(SerializableSettings { jobs: 2, color: true })
        .override_with(|settings| {
            settings.jobs = 4;
            settings.color = false;
        })
        .build()
        .expect("typed builder resolves settings");
    let serialized = serde_json::to_string(&built).expect("settings serialize");
    let roundtrip: SerializableSettings = serde_json::from_str(&serialized).expect("serialized settings deserialize");
    let missing_field_rejected = serde_json::from_str::<SerializableSettings>(r#"{"jobs":4}"#).is_err();

    assert_eq!(built.jobs, 4);
    assert!(!built.color);
    assert_eq!(roundtrip, built);
    assert!(missing_field_rejected);

    println!(
        "{{\"typed_fields\":2,\"builder_to_serialization_roundtrip\":1,\"missing_field_rejected\":{}}}",
        usize::from(missing_field_rejected)
    );
}

#[derive(conflaguration_derive::BorrowedSettings)]
#[borrowed_settings(crate = conflaguration_core)]
struct BorrowedTwoFields<'source> {
    workers: usize,
    color: &'source str,
}

#[derive(Settings)]
#[settings(prefix = "F6")]
struct ExistingTwoFields {
    workers: usize,
    color: String,
}

fn matches_existing_two_fields(candidate: &Result<BorrowedTwoFields<'_>, ResolveError>, existing: &conflaguration::Result<ExistingTwoFields>) -> bool {
    match (candidate, existing) {
        (Ok(candidate), Ok(existing)) => candidate.workers == existing.workers && candidate.color == existing.color,
        (Err(ResolveError::Invalid { field: "WORKERS" }), Err(conflaguration::Error::Env(error))) => {
            let message = error.to_string();
            message.contains("F6_WORKERS") && message.contains('x')
        }
        _ => false,
    }
}

#[test]
fn derive_two_fields() {
    let pairs = [("WORKERS", "2"), ("COLOR", "always")];
    let source = BorrowedSettings::new(&pairs);
    let candidate = BorrowedTwoFields::resolve_from(&source);
    let existing = temp_env::with_vars([("F6_WORKERS", Some("2")), ("F6_COLOR", Some("always"))], ExistingTwoFields::from_env);
    assert!(matches_existing_two_fields(&candidate, &existing));
    let resolved = candidate.expect("both borrowed fields resolve");
    assert_eq!(resolved.workers, 2);
    assert_eq!(resolved.color, "always");
    let owned_copies = usize::from(resolved.color.as_ptr() != pairs[1].1.as_ptr());
    assert_eq!(owned_copies, 0);

    let invalid_pairs = [("WORKERS", "x"), ("COLOR", "always")];
    let invalid_source = BorrowedSettings::new(&invalid_pairs);
    let invalid = BorrowedTwoFields::resolve_from(&invalid_source);
    let existing_invalid = temp_env::with_vars([("F6_WORKERS", Some("x")), ("F6_COLOR", Some("always"))], ExistingTwoFields::from_env);
    assert!(matches_existing_two_fields(&invalid, &existing_invalid));
    assert!(matches!(invalid, Err(ResolveError::Invalid { field: "WORKERS" })));

    let accepted_bad = Ok(BorrowedTwoFields {
        workers: 0,
        color: invalid_pairs[1].1,
    });
    let mutation_rejected = !matches_existing_two_fields(&accepted_bad, &existing_invalid);
    assert!(mutation_rejected);

    println!(
        "{{\"resolved_fields\":2,\"workers\":2,\"color\":\"always\",\"invalid_rejections\":1,\"comparator_matches\":2,\"mutation_rejected\":{},\"owned_copies\":{owned_copies}}}",
        usize::from(mutation_rejected)
    );
}

#[derive(Settings)]
#[settings(prefix = "F4")]
struct ScalarSettings {
    #[setting(default = 0)]
    workers: usize,
}

#[cfg(feature = "alloc")]
impl ExistingValidate for ScalarSettings {
    fn validate(&self) -> conflaguration::Result<()> {
        if self.workers == 0 {
            Err(conflaguration::Error::Validation {
                errors: vec![conflaguration::ValidationMessage::new("workers", "must be > 0")],
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "alloc")]
struct BorrowedWorkers {
    workers: usize,
}

#[cfg(feature = "alloc")]
impl BorrowedValidate for BorrowedWorkers {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.workers == 0 { Err(ValidationError::single("workers", "must be > 0")) } else { Ok(()) }
    }
}

#[cfg(feature = "alloc")]
fn matches_existing_validation(candidate: &Result<(), ValidationError>, existing: &conflaguration::Result<()>) -> bool {
    match (candidate, existing) {
        (Ok(()), Ok(())) => true,
        (Err(candidate), Err(existing_error @ conflaguration::Error::Validation { errors })) => {
            candidate.errors.len() == errors.len()
                && candidate
                    .errors
                    .iter()
                    .zip(errors)
                    .all(|(left, right)| left.path == right.path && left.message == right.message)
                && candidate.to_string() == existing_error.to_string()
        }
        _ => false,
    }
}

#[cfg(feature = "alloc")]
#[test]
fn validation_failure() {
    let pairs = [("WORKERS", "0")];
    let source = BorrowedSettings::new(&pairs);
    let workers = source
        .parse_scalar::<usize>("WORKERS")
        .expect("WORKERS=0 is a valid scalar")
        .expect("WORKERS pair is present");
    let candidate = BorrowedWorkers { workers }.validate();
    let existing = temp_env::with_vars([("F4_WORKERS", Some("0"))], || ScalarSettings::from_env().expect("WORKERS=0 resolves as a scalar").validate());
    assert!(matches_existing_validation(&candidate, &existing));

    let errors = &candidate.as_ref().expect_err("workers=0 must fail validation").errors;
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path, "workers");
    assert_eq!(errors[0].message, "must be > 0");
    let mutation_rejected = !matches_existing_validation(&Ok(()), &existing);
    assert!(mutation_rejected);

    let valid = BorrowedWorkers { workers: 2 }.validate();
    let existing_valid = temp_env::with_vars([("F4_WORKERS", Some("2"))], || ScalarSettings::from_env().expect("WORKERS=2 resolves as a scalar").validate());
    assert!(matches_existing_validation(&valid, &existing_valid));

    println!(
        "{{\"typed_validation_failures\":{},\"panics\":0,\"field\":\"{}\",\"message\":\"{}\",\"comparator_matches\":2,\"mutation_rejected\":{},\"valid_accepted\":1}}",
        errors.len(),
        errors[0].path,
        errors[0].message,
        usize::from(mutation_rejected)
    );
}

fn matches_existing_scalar(value: &str, parsed: &Result<Option<usize>, ParseIntError>) -> bool {
    temp_env::with_vars([("F4_WORKERS", Some(value))], || {
        let existing = ScalarSettings::from_env();
        match (parsed, existing) {
            (Ok(Some(workers)), Ok(settings)) => *workers == settings.workers,
            (Err(_), Err(_)) => true,
            _ => false,
        }
    })
}

#[test]
fn one_scalar() {
    let values = ["2", "7", "x"];
    let mut parsed = [0_usize; 2];
    let mut parsed_count = 0_usize;
    let mut invalid_rejected = 0_usize;
    let mut comparator_matches = 0_usize;
    let mut owned_copies = 0_usize;

    for value in values {
        let pairs = [("WORKERS", value)];
        let source = BorrowedSettings::new(&pairs);
        let result = source.parse_scalar::<usize>("WORKERS");
        owned_copies += usize::from(source.get("WORKERS").expect("WORKERS is present").as_ptr() != value.as_ptr());
        comparator_matches += usize::from(matches_existing_scalar(value, &result));

        match result {
            Ok(Some(workers)) => {
                parsed[parsed_count] = workers;
                parsed_count += 1;
            }
            Err(_) => invalid_rejected += 1,
            Ok(None) => panic!("WORKERS pair is present"),
        }
    }

    assert_eq!(parsed, [2, 7]);
    assert_eq!(parsed_count, 2);
    assert_eq!(invalid_rejected, 1);
    assert_eq!(comparator_matches, 3);
    assert_eq!(owned_copies, 0);
    let mutation_rejected = !matches_existing_scalar("x", &Ok(Some(0)));
    assert!(mutation_rejected);

    println!(
        "{{\"parsed\":[{},{}],\"parsed_count\":{parsed_count},\"invalid_rejected\":{invalid_rejected},\"comparator_matches\":{comparator_matches},\"mutation_rejected\":{},\"owned_copies\":{owned_copies}}}",
        parsed[0],
        parsed[1],
        usize::from(mutation_rejected)
    );
}

struct MissingAsPresent<'source>(&'source BorrowedSettings<'source>);

impl SettingsSource for MissingAsPresent<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        if key == "MISSING" { Some("always") } else { self.0.get(key) }
    }
}

fn matches_literal(source: &impl SettingsSource) -> bool {
    source.get("COLOR") == Some("always") && source.get("MISSING").is_none()
}

#[test]
fn one_key() {
    let pairs = [("COLOR", "always")];
    let source = BorrowedSettings::new(&pairs);
    let present = source.get("COLOR");
    let missing = source.get("MISSING");
    assert_eq!(present, Some("always"));
    assert_eq!(missing, None);
    assert!(matches_literal(&source));
    assert!(!matches_literal(&MissingAsPresent(&source)));
    let owned_copies = usize::from(present.expect("COLOR is present").as_ptr() != pairs[0].1.as_ptr());
    assert_eq!(owned_copies, 0);
    println!("{{\"present\":1,\"absent\":1,\"owned_copies\":{owned_copies},\"mutated_control_rejected\":1}}");
}

fn matches_os(state: SettingState<'_>, observed: Option<&OsStr>) -> bool {
    match (state, observed) {
        (SettingState::Absent, None) => true,
        (SettingState::PresentEmpty, Some(value)) => value.is_empty(),
        (SettingState::PresentNonempty(value), Some(observed)) => observed == OsStr::new(value),
        _ => false,
    }
}

#[test]
fn present_empty_absent() {
    temp_env::with_vars([("MISSING", None), ("EMPTY", Some("")), ("NONEMPTY", Some("x"))], || {
        let pairs = [("EMPTY", ""), ("NONEMPTY", "x")];
        let source = BorrowedSettings::new(&pairs);
        let states = [source.state("MISSING"), source.state("EMPTY"), source.state("NONEMPTY")];
        let expected = [SettingState::Absent, SettingState::PresentEmpty, SettingState::PresentNonempty("x")];
        assert_eq!(states, expected);
        assert_ne!(states[0], states[1]);
        assert_ne!(states[1], states[2]);
        assert_ne!(states[0], states[2]);

        let observed: [Option<OsString>; 3] = [std::env::var_os("MISSING"), std::env::var_os("EMPTY"), std::env::var_os("NONEMPTY")];
        let std_control_matches = states
            .iter()
            .zip(&observed)
            .filter(|(state, value)| matches_os(**state, value.as_deref()))
            .count();
        assert_eq!(std_control_matches, 3);

        let collapsed = [SettingState::PresentEmpty, SettingState::PresentEmpty, SettingState::PresentNonempty("x")];
        let collapsed_matches = collapsed
            .iter()
            .zip(&observed)
            .filter(|(state, value)| matches_os(**state, value.as_deref()))
            .count();
        assert_eq!(collapsed_matches, 2);
        let owned_copies = usize::from(source.get("NONEMPTY").expect("nonempty value is present").as_ptr() != pairs[1].1.as_ptr());
        assert_eq!(owned_copies, 0);
        println!(
            "{{\"distinct_states\":3,\"collapsed_states\":0,\"std_control_matches\":{std_control_matches},\"collapsed_control_matches\":{collapsed_matches},\"mutation_rejected\":1,\"owned_copies\":{owned_copies}}}"
        );
    });
}
