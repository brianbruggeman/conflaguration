//! C3 adversarial pass (task 5a): does a multi-layer file/overlay merge chain
//! produce a value that differs anywhere from what a single-shot deserialize
//! would produce for the same effective inputs — including a `#[serde(skip)]`
//! field and a nested sub-struct, which a merge medium keyed purely on the
//! wire representation could plausibly mishandle.

#![cfg(all(any(feature = "default", feature = "std"), feature = "toml"))]

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct Nested {
    inner_a: u32,
    inner_b: String,
}

#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct WithSkipAndNested {
    host: String,
    port: u16,
    nested: Nested,
    #[serde(skip)]
    runtime_only: u64,
}

#[test]
fn multi_layer_merge_preserves_skip_field_and_nested_substruct() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));

    let base_path = dir.path().join("base.toml");
    std::fs::write(&base_path, "host = \"base.com\"\nport = 1000\n\n[nested]\ninner_a = 1\ninner_b = \"one\"\n").unwrap_or_else(|err| panic!("write base failed: {err}"));

    let overlay_path = dir.path().join("overlay.toml");
    std::fs::write(&overlay_path, "port = 2000\n").unwrap_or_else(|err| panic!("write overlay failed: {err}"));

    let override_path = dir.path().join("override.toml");
    std::fs::write(&override_path, "\n[nested]\ninner_b = \"three\"\n").unwrap_or_else(|err| panic!("write override failed: {err}"));

    // three chained layers: base is the first source (direct deserialize, no
    // merge medium at all); overlay and override each force a merge-medium
    // round trip, exercising exactly the code path C3 targets.
    let seed = WithSkipAndNested {
        runtime_only: 42,
        ..Default::default()
    };

    let merged: WithSkipAndNested = conflaguration::builder()
        .value(seed)
        .file(&base_path)
        .overlay_file(&overlay_path)
        .file(&override_path)
        .build()
        .unwrap_or_else(|err| panic!("build failed: {err}"));

    assert_eq!(merged.host, "base.com", "base sets host; later layers never touch it");
    assert_eq!(merged.port, 1000, "base already sets a non-empty port, so the overlay layer must not clobber it");
    assert_eq!(merged.nested.inner_a, 1, "override layer omits inner_a; base's value survives the merge");
    assert_eq!(merged.nested.inner_b, "three", "override layer replaces inner_b under Override mode");
    assert_eq!(
        merged.runtime_only, 0,
        "#[serde(skip)] field is not part of the wire representation the merge medium carries; \
         it resets to its Default on every deserialize through that medium, by serde's own contract \
         for skipped fields — not a merge-cache artifact"
    );
}
