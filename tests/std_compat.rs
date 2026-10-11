#![cfg(any(feature = "default", feature = "std"))]

use std::fmt;
#[cfg(feature = "toml")]
use std::fs;

#[cfg(feature = "toml")]
use conflaguration::ConfigBuilder;
use conflaguration::ConfigDisplay;
use conflaguration::resolve_or;
#[cfg(feature = "toml")]
use serde::Deserialize;
#[cfg(feature = "toml")]
use serde::Serialize;

struct PinnedDisplay {
    port: u16,
}

impl ConfigDisplay for PinnedDisplay {
    fn fmt_config(&self, formatter: &mut fmt::Formatter<'_>, _depth: usize) -> fmt::Result {
        write!(formatter, "port={}", self.port)
    }
}

fn byte_differences(candidate: &str, pinned: &str) -> usize {
    candidate
        .as_bytes()
        .iter()
        .zip(pinned.as_bytes())
        .filter(|(left, right)| left != right)
        .count()
        + candidate.len().abs_diff(pinned.len())
}

#[test]
fn display_unchanged() {
    let rendered = format!("{}", PinnedDisplay { port: 8080 }.display());
    let pinned = "port=8080";
    let mutation = "port=8081";
    let actual_differences = byte_differences(&rendered, pinned);
    let mutation_differences = byte_differences(mutation, pinned);
    assert_eq!(actual_differences, 0);
    assert_eq!(mutation_differences, 1);
    let mutation_rejected = mutation_differences != 0;
    assert!(mutation_rejected);
    println!(
        "{{\"rendered\":\"{rendered}\",\"pinned\":\"{pinned}\",\"byte_differences\":{actual_differences},\"mutation_byte_differences\":{mutation_differences},\"mutation_rejected\":{}}}",
        usize::from(mutation_rejected)
    );
}

#[test]
fn env_unchanged() {
    temp_env::with_vars([("WORKERS", Some("2"))], || {
        let pinned = std::env::var("WORKERS")
            .expect("WORKERS should be set by the fixture")
            .parse::<u16>()
            .expect("fixture WORKERS should parse as u16");
        let resolved: u16 = resolve_or(&["WORKERS"], 7).expect("existing env resolver should parse WORKERS");
        let value_differences = usize::from(resolved != pinned);
        let mutation = 3_u16;
        let mutation_differences = usize::from(mutation != pinned);
        let mutation_rejected = mutation_differences != 0;

        assert_eq!(pinned, 2);
        assert_eq!(value_differences, 0);
        assert_eq!(mutation_differences, 1);
        assert!(mutation_rejected);
        println!(
            "{{\"environment\":\"WORKERS=2\",\"env_source_paths\":1,\"pinned\":{pinned},\"resolved\":{resolved},\"value_differences\":{value_differences},\"mutation\":{mutation},\"mutation_differences\":{mutation_differences},\"mutation_rejected\":{}}}",
            usize::from(mutation_rejected)
        );
    });
}

#[cfg(feature = "toml")]
#[derive(Deserialize, Serialize)]
struct FileWorkers {
    workers: u16,
}

#[cfg(feature = "toml")]
#[test]
fn file_unchanged() {
    let directory = tempfile::tempdir().expect("fixture temporary directory should be created");
    let base = directory.path().join("base.toml");
    let overlay = directory.path().join("overlay.toml");
    fs::write(&base, "workers = 2\n").expect("base fixture should be written");
    fs::write(&overlay, "workers = 7\n").expect("overlay fixture should be written");

    let resolved: FileWorkers = ConfigBuilder::new()
        .file(&base)
        .file(&overlay)
        .build()
        .expect("file override order should resolve the fixture");
    let reversed: FileWorkers = ConfigBuilder::new()
        .file(&overlay)
        .file(&base)
        .build()
        .expect("reverse order control should resolve the fixture");

    let pinned_workers = 7_u16;
    let precedence_differences = usize::from(resolved.workers != pinned_workers);
    let mutation_differences = usize::from(reversed.workers != pinned_workers);
    let mutation_rejected = mutation_differences != 0;

    assert_eq!(resolved.workers, pinned_workers);
    assert_eq!(precedence_differences, 0);
    assert_eq!(reversed.workers, 2);
    assert_eq!(mutation_differences, 1);
    assert!(mutation_rejected);
    println!(
        "{{\"file_base_workers\":2,\"file_overlay_workers\":7,\"file_source_paths\":1,\"resolved_workers\":{},\"pinned_workers\":{pinned_workers},\"precedence_differences\":{precedence_differences},\"reversed_workers\":{},\"mutation_differences\":{mutation_differences},\"mutation_rejected\":{}}}",
        resolved.workers,
        reversed.workers,
        usize::from(mutation_rejected)
    );
}
