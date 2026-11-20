#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;

    use serde_json::Value;

    const TARGET: &str = "x86_64-unknown-none";

    fn run(command: &mut Command) -> std::process::Output {
        let output = command.output().expect("probe command should start");
        assert!(
            output.status.success(),
            "probe command failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn library_artifact(output: &[u8], target_name: &str) -> PathBuf {
        let mut matches = Vec::new();
        for line in output.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()) {
            let record: Value = serde_json::from_slice(line).expect("Cargo JSON record should parse");
            if record["reason"] != "compiler-artifact" || record["target"]["name"] != target_name {
                continue;
            }
            let Some(filenames) = record["filenames"].as_array() else {
                continue;
            };
            for filename in filenames {
                if let Some(filename) = filename.as_str().filter(|name| name.ends_with(".rmeta")) {
                    matches.push(PathBuf::from(filename));
                }
            }
        }
        assert_eq!(matches.len(), 1, "expected one target library artifact");
        matches.remove(0)
    }

    // cargo's build-dir layout keeps dependency artifacts under build/<pkg>/<hash>/out, not deps/, so search dirs come from the artifact records
    fn dependency_dirs(output: &[u8]) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for line in output.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()) {
            let record: Value = serde_json::from_slice(line).expect("Cargo JSON record should parse");
            let filenames = record["filenames"].as_array().into_iter().flatten().filter_map(Value::as_str);
            for parent in filenames.filter_map(|name| Path::new(name).parent()) {
                if !dirs.iter().any(|dir: &PathBuf| dir == parent) {
                    dirs.push(parent.to_path_buf());
                }
            }
        }
        dirs
    }

    fn compile_fixture(source: &Path, artifact: &Path, search_dirs: &[PathBuf], crate_name: &str, output_path: &Path) -> std::process::Output {
        let mut command = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        for dir in search_dirs {
            command.arg("-L").arg(format!("dependency={}", dir.display()));
        }
        command
            .arg("--edition=2024")
            .arg("--crate-type=lib")
            .arg("--target")
            .arg(TARGET)
            .arg("--emit=metadata")
            .arg("--extern")
            .arg(format!("{crate_name}={}", artifact.display()))
            .arg("-o")
            .arg(output_path)
            .arg(source)
            .output()
            .expect("fixture rustc should start")
    }

    #[test]
    fn no_std_export() {
        let package = env::var("F9_CHECK_PACKAGE").unwrap_or_else(|_| "conflaguration".to_owned());
        let manifest = env::var_os("F9_CHECK_MANIFEST")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
        let crate_name = package.replace('-', "_");
        let directory = tempfile::tempdir().expect("probe directory should be created");
        let target_dir = env::var_os("F9_PROBE_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| directory.path().join("target"));

        let mut tree_command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        tree_command
            .arg("tree")
            .arg("-p")
            .arg(&package)
            .arg("--manifest-path")
            .arg(&manifest)
            .arg("--no-default-features")
            .arg("--target")
            .arg(TARGET)
            .arg("-e")
            .arg("normal,features")
            .arg("--prefix")
            .arg("depth");
        if package == "conflaguration" {
            tree_command.arg("--features").arg("derive");
        }
        let tree = run(&mut tree_command);
        let tree_text = String::from_utf8(tree.stdout).expect("Cargo tree should be UTF-8");
        let std_edges = tree_text.lines().filter(|line| line.contains(" feature \"std\"")).count();
        assert_eq!(std_edges, 0, "normal feature graph contains std: {tree_text}");

        let mut check_command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        check_command
            .arg("check")
            .arg("-p")
            .arg(&package)
            .arg("--manifest-path")
            .arg(&manifest)
            .arg("--no-default-features")
            .arg("--lib")
            .arg("--target")
            .arg(TARGET)
            .arg("--message-format=json")
            .env("CARGO_TARGET_DIR", &target_dir);
        if package == "conflaguration" {
            check_command.arg("--features").arg("derive");
        }
        let check = run(&mut check_command);
        let artifact = library_artifact(&check.stdout, &crate_name);
        let search_dirs = dependency_dirs(&check.stdout);

        let source = directory.path().join("consumer.rs");
        let accepted = if package == "conflaguration" {
            "#![no_std]\nuse conflaguration::{BorrowedSettings, ResolveFromSource};\n#[derive(conflaguration::BorrowedSettings)]\nstruct Fixture<'source> { workers: usize, color: &'source str }\npub fn workers() -> usize { let pairs = [(\"WORKERS\", \"2\"), (\"COLOR\", \"always\")]; let source = BorrowedSettings::new(&pairs); Fixture::resolve_from(&source).map_or(0, |settings| settings.workers) }\n"
        } else {
            "#![no_std]\nuse proxima_primitives::pipe::Pipe;\npub fn accept<T: Pipe>(_: &T) {}\n"
        };
        fs::write(&source, accepted).expect("accepted fixture should be written");
        let accepted_output = compile_fixture(&source, &artifact, &search_dirs, &crate_name, &directory.path().join("accepted.rmeta"));
        assert!(accepted_output.status.success(), "no_std export fixture failed: {}", String::from_utf8_lossy(&accepted_output.stderr));

        fs::write(&source, format!("{accepted}\nuse std::env;\npub fn rejected() {{ let _ = env::var(\"WORKERS\"); }}\n")).expect("mutation fixture should be written");
        let rejected_output = compile_fixture(&source, &artifact, &search_dirs, &crate_name, &directory.path().join("rejected.rmeta"));
        let rejected_stderr = String::from_utf8_lossy(&rejected_output.stderr);
        assert!(!rejected_output.status.success(), "std::env mutation unexpectedly compiled");
        assert!(rejected_stderr.contains("std"), "rejection did not identify std: {rejected_stderr}");

        println!("{{\"package\":\"{package}\",\"std_feature_edges\":{std_edges},\"library_artifacts\":1,\"exports_compiled\":1,\"std_env_mutation_rejections\":1}}");
    }
}
