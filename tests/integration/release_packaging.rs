use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn repo_root() -> PathBuf {
    env::var("HLS_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn read(path: &str) -> String {
    let full_path = repo_root().join(path);
    fs::read_to_string(&full_path).unwrap_or_else(|err| {
        panic!("read {}: {err}", full_path.display());
    })
}

fn assert_relative_markdown_links_exist(paths: &[&str]) {
    for path in paths {
        let document = read(path);
        let parent = repo_root()
            .join(path)
            .parent()
            .expect("document parent")
            .to_path_buf();
        let mut remainder = document.as_str();
        while let Some(marker) = remainder.find("](") {
            remainder = &remainder[marker + 2..];
            let Some(end) = remainder.find(')') else {
                panic!("unterminated Markdown link in {path}");
            };
            let raw_target = remainder[..end].trim().trim_matches(['<', '>']);
            remainder = &remainder[end + 1..];
            if raw_target.is_empty()
                || raw_target.starts_with('#')
                || raw_target.starts_with("http://")
                || raw_target.starts_with("https://")
                || raw_target.starts_with("mailto:")
            {
                continue;
            }
            let target = raw_target.split('#').next().unwrap_or_default();
            assert!(
                parent.join(target).exists(),
                "broken relative Markdown link in {path}: {raw_target}",
            );
        }
    }
}

#[test]
fn distributable_crates_are_not_publishable() {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repo_root())
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata");
    let packages = metadata["packages"]
        .as_array()
        .expect("cargo metadata packages array");
    let workspace_members = metadata["workspace_members"]
        .as_array()
        .expect("cargo metadata workspace members array");
    let mut package_ids: Vec<_> = packages
        .iter()
        .map(|package| package["id"].as_str().expect("package id"))
        .collect();
    let mut workspace_member_ids: Vec<_> = workspace_members
        .iter()
        .map(|member| member.as_str().expect("workspace member id"))
        .collect();
    package_ids.sort_unstable();
    workspace_member_ids.sort_unstable();
    assert_eq!(
        package_ids, workspace_member_ids,
        "metadata packages must cover every workspace member",
    );

    let publishable: Vec<_> = packages
        .iter()
        .filter(|package| package["publish"] != serde_json::json!([]))
        .map(|package| package["name"].as_str().unwrap_or("<unnamed>"))
        .collect();
    assert!(
        publishable.is_empty(),
        "workspace packages must report publish == []: {publishable:?}",
    );
}

#[test]
fn dist_workspace_declares_tag_gated_release_artifacts() {
    let dist = read("dist-workspace.toml");

    assert!(dist.contains("[dist]"));
    assert!(dist.contains("ci = \"github\""));
    assert!(dist.contains("pr-run-mode = \"upload\""));
    assert!(dist.contains("allow-dirty = [\"ci\"]"));
    assert!(dist.contains("cache-builds = false"));
    assert!(dist.contains("hosting = \"github\""));
    assert!(dist.contains("github-attestations = true"));
    assert!(dist.contains("install-updater = false"));
    assert!(dist.contains("THIRD_PARTY_LICENSES.txt"));
    assert!(dist.contains("THIRD_PARTY_NOTICES.md"));
    assert!(dist.contains("NOTICE"));
    let cargo = read("Cargo.toml");
    assert!(cargo.contains("[profile.dist]"));
    assert!(cargo.contains("inherits = \"release\""));
}

#[test]
fn dependency_attribution_is_deterministic_and_matches_release_targets() {
    let about = read("about.toml");
    let deny = read("deny.toml");
    let template = read("about.hbs");
    let checker = read("scripts/check-third-party-licenses.sh");
    let ci = read(".github/workflows/ci.yml");
    let attributes = read(".gitattributes");

    for target in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
    ] {
        assert!(about.contains(target), "about.toml is missing {target}");
        assert!(deny.contains(target), "deny.toml is missing {target}");
    }
    assert!(about.contains("ignore-dev-dependencies = true"));
    assert!(!about.contains("no-clearly-defined ="));
    assert!(template.contains("{{crate.name}} {{crate.version}}"));
    assert!(template.contains("{{#each licenses}}"));
    assert!(template.contains("{{{text}}}"));
    assert!(checker.contains("cargo about generate"));
    assert!(checker.contains("--workspace"));
    assert!(checker.contains("--all-features"));
    assert!(checker.contains("--locked"));
    assert!(checker.contains("--offline"));
    assert!(checker.contains("--output-file \"$generated\""));
    assert!(checker.contains("--fail"));
    assert!(checker.contains("--config about.toml"));
    assert!(checker.contains("cmp"));
    assert!(!checker.contains("--frozen"));
    assert!(checker.contains("cargo-about 0.9.1"));
    assert!(checker.contains("scripts/check-third-party-notices.py"));
    assert!(ci.contains("cargo install cargo-about --version 0.9.1 --locked --features cli"));
    assert!(ci.contains("scripts/check-third-party-licenses.sh"));
    assert!(attributes.contains("THIRD_PARTY_LICENSES.txt -text"));
    assert!(attributes.contains("whitespace=-trailing-space,-cr-at-eol"));
}

#[test]
fn third_party_notices_cover_packaged_dependency_notices() {
    let notice = read("THIRD_PARTY_NOTICES.md");
    let notice_manifest = read("third_party/notices/manifest.json");
    let parquet_notice = read("third_party/notices/parquet-59.1.0-NOTICE.txt");
    let notice_checker = read("scripts/check-third-party-notices.py");

    assert!(notice.contains("Apache Arrow"));
    assert!(notice.contains("Copyright 2016-2026 The Apache Software Foundation"));
    assert!(notice.contains("https://github.com/olliemath/chronoutil"));
    assert!(notice.contains("https://github.com/jhorstmann/compact-thrift"));
    assert!(notice.contains("third_party/notices/parquet-59.1.0-NOTICE.txt"));
    assert!(notice_manifest.contains("\"package\": \"parquet\""));
    assert!(notice_manifest.contains("\"version\": \"59.1.0\""));
    assert!(notice_manifest.contains("\"source\": \"NOTICE.txt\""));
    assert!(notice_manifest.contains("\"package\": \"cfg_aliases\""));
    assert!(notice_manifest.contains("\"version\": \"0.1.1\""));
    assert!(notice_manifest.contains("\"version\": \"0.2.1\""));
    assert!(notice_manifest.contains("\"source\": \"NOTICES.md\""));
    assert!(parquet_notice.contains("Apache Arrow"));
    assert!(notice_checker.contains("\"cargo\","));
    assert!(notice_checker.contains("\"metadata\","));
    assert!(notice_checker.contains("\"--all-features\","));
    assert!(notice_checker.contains("untracked packaged NOTICE files"));
    assert!(notice_checker.contains("does not match its packaged source"));
}

#[test]
fn local_release_archive_stages_project_and_third_party_notices() {
    let local_smoke = read("scripts/local-release-artifact-smoke.sh");
    let releasing = read("docs/RELEASING.md");

    for file in [
        "LICENSE",
        "NOTICE",
        "THIRD_PARTY_LICENSES.txt",
        "THIRD_PARTY_NOTICES.md",
    ] {
        assert!(
            local_smoke.contains(&format!("$repo_root/{file}")),
            "local archive does not stage {file}",
        );
        assert!(
            local_smoke.contains(&format!("$unpack_dir/$package_name/{file}")),
            "local archive does not validate unpacked {file}",
        );
        assert!(
            local_smoke.contains(&format!("test -s \"$unpack_dir/$package_name/{file}\"")),
            "local archive does not require non-empty {file}",
        );
        assert!(releasing.contains(file), "release docs omit {file}");
    }
}

#[test]
fn maintainer_journals_and_agent_state_are_not_tracked() {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("run git ls-files");
    assert!(output.status.success(), "git ls-files failed");
    let tracked = String::from_utf8_lossy(&output.stdout);

    let blocked_prefixes = [
        ".agents/",
        ".claude/",
        ".codex/",
        ".cursor/",
        ".specify/",
        ".superpowers/",
        "memory/",
        "plans/",
        "reflections/",
        "specs/",
        "docs/agent-memory/",
        "docs/superpowers/",
        "docs/reports/",
        "third_party/spec-kit/",
    ];
    let blocked_files = [
        "AGENTS.md",
        "MEMORY.md",
        "PLAN.md",
        "TODO.md",
        "SOUL.md",
        "USER.md",
        "docs/OPEN_SOURCE_AUDIT.md",
        "docs/OPEN_SOURCE_CHECKLIST.md",
        "docs/DEVELOPMENT_TOOLING.md",
    ];
    for path in tracked.split('\0').filter(|path| !path.is_empty()) {
        assert!(
            !blocked_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix)),
            "development-assistant or journal path is tracked: {path}",
        );
        assert!(
            !blocked_files.contains(&path),
            "development-assistant or journal file is tracked: {path}",
        );
    }

    let ignore = read(".gitignore");
    for entry in [".cursor/", ".specify/", "/reflections/", "/PLAN.md"] {
        assert!(
            ignore.contains(entry),
            ".gitignore lost the local-state guard for {entry}",
        );
    }
}

#[test]
fn release_workflow_is_pull_request_plan_and_tag_publish_only() {
    let workflow = read(".github/workflows/release.yml");

    assert!(workflow.contains("This file was autogenerated by dist"));
    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("push:"));
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("if [[ \"$DIST_PUBLISHING\" == \"true\" ]]"));
    assert!(workflow.contains("dist host --steps=create --tag=\"$DIST_TAG\""));
    assert!(workflow.contains("dist plan --output-format=json"));
    assert!(!workflow.contains("dist ${{ (!github.event.pull_request"));
    assert!(workflow.contains("dist build"));
    assert!(workflow.contains("dist host"));
    assert!(workflow.contains("Create GitHub Release"));
    assert!(workflow.contains("needs.plan.outputs.publishing == 'true'"));
    assert!(!workflow.contains("cargo dist plan"));
    assert!(!workflow.contains("cargo dist build"));
    assert!(!workflow.contains("pull_request_target"));

    let forbidden_secret_refs: Vec<_> = workflow
        .lines()
        .filter(|line| line.contains("secrets.") && !line.contains("secrets.GITHUB_TOKEN"))
        .collect();
    assert!(
        forbidden_secret_refs.is_empty(),
        "unexpected release secret refs: {forbidden_secret_refs:?}",
    );
}

#[test]
fn workflows_pin_runners_actions_and_release_permissions() {
    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("schedule:"));
    assert!(ci.contains("ubuntu-24.04"));
    assert!(ci.contains("macos-15"));
    assert!(!ci.contains("ubuntu-latest"));
    assert!(!ci.contains("macos-latest"));
    assert!(ci.contains("public-contract-smoke"));
    assert!(ci.contains("--all-symbols"));
    assert!(ci.contains("fetch-depth: 0"));
    assert!(ci.contains("zizmor@1.26.1"));
    assert!(ci.contains("--pedantic"));
    assert!(ci.contains("--strict-collection"));
    assert!(ci.contains("cargo install cargo-deny --version 0.20.2 --locked"));
    assert!(ci.contains("cargo deny check licenses sources"));

    let release = read(".github/workflows/release.yml");
    assert!(release.contains("permissions:\n  \"contents\": \"read\""));
    let host = release
        .split("  host:")
        .nth(1)
        .expect("release workflow has host job");
    assert!(host.contains("\"contents\": \"write\""));
    assert!(host.contains("\"attestations\": \"write\""));
    assert!(host.contains("\"id-token\": \"write\""));

    for path in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let workflow = read(path);
        assert_eq!(
            workflow.matches("actions/checkout@").count(),
            workflow.matches("persist-credentials: false").count(),
            "every checkout must disable credential persistence in {path}",
        );
        for line in workflow
            .lines()
            .filter(|line| line.trim().starts_with("uses:"))
        {
            let reference = line
                .split_once('@')
                .map(|(_, reference)| reference.trim())
                .unwrap_or_default()
                .split_whitespace()
                .next()
                .unwrap_or_default();
            assert_eq!(
                reference.len(),
                40,
                "action is not SHA-pinned in {path}: {line}"
            );
            assert!(
                reference
                    .chars()
                    .all(|character| character.is_ascii_hexdigit()),
                "action has a non-hex pin in {path}: {line}",
            );
        }
    }
}

#[test]
fn dist_release_contract_builds_pr_artifacts_sbom_and_provenance() {
    let dist = read("dist-workspace.toml");
    assert!(dist.contains("pr-run-mode = \"upload\""));
    assert!(dist.contains("source-tarball = true"));
    assert!(dist.contains("cargo-cyclonedx = true"));
    assert!(dist.contains("cargo-auditable = true"));
    assert!(dist.contains("github-attestations = true"));
    assert!(dist.contains("github-attestations-phase = \"host\""));
    assert!(dist.contains("[dist.github-action-commits]"));

    let release = read(".github/workflows/release.yml");
    assert!(release.contains("dist build"));
    assert!(release.contains("Attest"));
    assert!(release.contains("*.cdx.xml"));
    assert!(release.contains("steps.cargo-cyclonedx.outputs.paths"));
    assert!(!release.contains("steps.cargo-cyclonedx.output.paths"));
    assert!(release.contains("cargo-dist 0.32.0 requires reviewed post-generation security fixes"));
    assert!(release.contains("cargo install cargo-dist --version 0.32.0 --locked"));
    assert!(release.contains("cargo install cargo-auditable --version 0.7.5 --locked"));
    assert!(release.contains("cargo install cargo-audit --version 0.22.2 --locked"));
    assert!(release.contains("cargo install cargo-cyclonedx --version 0.5.5 --locked"));
    assert!(release.contains("scripts/verify-dist-local-artifact.py"));
    assert!(release.contains(
        "      matrix: ${{ fromJson(needs.plan.outputs.val).ci.github.artifacts_matrix }}"
    ));
    assert_eq!(release.matches("timeout-minutes:").count(), 5);
    assert!(!release.contains("| sh"));
    assert!(!release.contains("| iex"));
    assert!(release.contains("artifacts/*.sha256"));
    assert!(release.contains("needs.plan.outputs.publishing == 'true'"));
    assert!(!release.contains("pull_request_target"));
    assert!(!release.contains("swatinem/rust-cache"));
    assert!(!release.contains("container: ${{"));
    assert!(!release.contains("run: ${{"));
    assert!(!release.contains("${{ matrix.packages_install }}"));
}

#[test]
fn distributable_crate_inherits_public_repository_metadata() {
    let manifest = read("crates/hls-cli/Cargo.toml");

    assert!(manifest.contains("authors.workspace = true"));
    assert!(manifest.contains("repository.workspace = true"));
    assert!(manifest.contains("homepage.workspace = true"));
    assert!(manifest.contains("description.workspace = true"));
    assert!(manifest.contains("[package.metadata.dist]"));
    assert!(manifest.contains("dist = true"));
}

#[test]
fn release_docs_explain_local_dry_run_and_no_secrets_boundary() {
    let docs = read("docs/RELEASING.md");

    assert!(docs.contains("dist plan"));
    assert!(docs.contains("dist build"));
    assert!(!docs.contains("cargo dist plan"));
    assert!(!docs.contains("cargo dist build"));
    assert!(docs.contains("No release secrets"));
    assert!(docs.contains("git tag -a v"));
}

#[test]
fn release_validation_scripts_cover_local_artifacts_checksums_and_public_readiness() {
    let local_smoke = read("scripts/local-release-artifact-smoke.sh");
    assert!(local_smoke.contains("target/release/hls"));
    assert!(local_smoke.contains("tar -czf"));
    assert!(local_smoke.contains("sha256"));
    assert!(local_smoke.contains("doctor --data-dir"));
    assert!(local_smoke.contains("--fixture-file"));
    assert!(!local_smoke.contains("git push"));
    assert!(!local_smoke.contains("gh release upload"));

    let public_scan = read("scripts/check-public-readiness.sh");
    assert!(public_scan.contains("README.md"));
    assert!(public_scan.contains("deny.toml"));
    assert!(public_scan.contains("SECURITY.md"));
    assert!(public_scan.contains("docs/ROADMAP.md"));
    assert!(public_scan.contains("docs/assets/screenshots/live-screen.svg"));
    assert!(public_scan.contains("docs/evidence/soak/sota-allpairs-20260720-15m.json"));
    assert!(public_scan.contains("scripts/harden-generated-release-workflow.py"));
    assert!(public_scan.contains("private_path_pattern"));
    assert!(public_scan.contains("credential_pattern"));
    assert!(public_scan.contains("git grep -n -E -e \"$private_path_pattern\""));
    assert!(public_scan.contains("git grep -l -E -e \"$credential_pattern\""));
    assert!(public_scan.contains("credential_status > 1"));
    assert!(public_scan.contains("scripts/check-history-secrets.sh"));
    assert!(!public_scan.contains("history.patch"));

    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("zizmor@1.26.1"));

    let packaging_check = read("scripts/check-release-packaging.sh");
    assert!(packaging_check.contains("check-public-readiness.sh"));
    assert!(packaging_check.contains("local-release-artifact-smoke.sh"));
    assert!(packaging_check.contains("harden-generated-release-workflow.py"));
    assert!(packaging_check.contains("--check"));
    let hardener = read("scripts/harden-generated-release-workflow.py");
    assert!(hardener.contains("--regenerate"));
    assert!(hardener.contains("dist-workspace.toml"));
    assert!(hardener.contains("finally:"));
    assert!(packaging_check.contains("validate-soak-report.py"));
    assert!(packaging_check.contains("soak-report-valid.json"));
    assert!(packaging_check.contains("sota-allpairs-20260720-15m.json"));
    assert!(!packaging_check.contains("merge-base --is-ancestor"));
    assert!(packaging_check.contains("soak-report-invalid.json"));
    assert!(packaging_check.contains("soak-report-invalid-command.json"));
    assert!(packaging_check.contains("check-supervisor-packaging.sh"));
    assert!(packaging_check.contains("runtime-source-sha256.py"));
    assert!(packaging_check.contains("runtime_source_sha256"));
    assert!(packaging_check.contains("soak_evidence_binary_match"));
    assert!(packaging_check.contains("--binary"));
}

#[test]
fn release_docs_and_roadmap_separate_local_proof_from_publication() {
    let releasing = read("docs/RELEASING.md");
    assert!(releasing.contains("Local Artifact Smoke"));
    assert!(releasing.contains("Artifact Checklist"));
    assert!(releasing.contains("Release Artifact Status"));
    assert!(releasing.contains("not a published release"));
    assert!(releasing.contains("not a supported long-running daemon"));

    let roadmap = read("docs/ROADMAP.md");
    assert!(roadmap.contains("Draft/local proof only"));
    assert!(roadmap.contains("no reviewed `v*` release artifact publication"));
    assert!(roadmap.contains("These are not a supported production service"));
    assert!(
        roadmap
            .contains("Validate supervisor templates before describing them as deployment support")
    );
}

#[test]
fn rustsec_gate_keeps_one_documented_transitive_warning_exception() {
    let expected = "cargo audit --deny warnings --ignore RUSTSEC-2024-0436";
    let workflow = read(".github/workflows/ci.yml");
    let releasing = read("docs/RELEASING.md");
    let readiness = read("docs/production-readiness.md");

    assert!(workflow.contains(expected));
    assert!(releasing.contains(expected));
    assert!(readiness.contains(expected));
    assert!(workflow.contains("Apache Parquet 59.1.0"));
    assert!(releasing.contains("all other warnings remain denied"));
    assert!(readiness.contains("all other dependency warnings remain denied"));
}

#[test]
fn canonical_validation_entrypoint_has_bounded_modes_and_ci_doc_parity() {
    let check = read("scripts/check.sh");

    assert!(check.contains("mode=\"${1:-pr}\""));
    assert!(check.contains("fast|pr|release)"));
    assert!(check.contains("Usage: scripts/check.sh [fast|pr|release]"));
    assert!(check.contains("run_fast_checks"));
    assert!(check.contains("run_pr_checks"));
    assert!(check.contains("run_release_checks"));
    assert!(check.contains("cargo check --workspace --all-features --locked"));
    assert!(check.contains("cargo test --workspace --all-features --locked"));
    assert!(check.contains("cargo fmt --all -- --check"));
    assert!(
        check.contains(
            "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
        )
    );
    assert!(check.contains("cargo build --release --workspace --all-features --locked"));
    assert!(check.contains(
        "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --all-features --no-deps --locked"
    ));
    assert!(check.contains("python3 scripts/generate-screenshots.py --check"));
    assert!(check.contains("scripts/check-release-packaging.sh"));
    assert!(check.contains("git diff --check"));
    assert!(check.contains("cargo-audit 0.22.2"));
    assert!(check.contains("cargo-audit --version"));
    assert!(check.contains("cargo audit --deny warnings --ignore RUSTSEC-2024-0436"));
    assert!(check.contains("cargo-deny 0.20.2"));
    assert!(check.contains("cargo deny check licenses sources"));
    assert!(check.contains("scripts/check-third-party-licenses.sh"));
    assert!(check.contains("uvx \"zizmor@1.26.1\""));
    assert!(!check.contains("--offline"));
    assert!(check.contains("--pedantic"));
    assert!(check.contains("--strict-collection"));

    let unknown = Command::new("bash")
        .args(["scripts/check.sh", "unknown"])
        .current_dir(repo_root())
        .output()
        .expect("run validation entrypoint with unknown mode");
    assert!(
        !unknown.status.success(),
        "unknown mode unexpectedly passed"
    );
    assert!(
        String::from_utf8_lossy(&unknown.stderr)
            .contains("Usage: scripts/check.sh [fast|pr|release]"),
        "unknown mode did not print the bounded usage contract: {}",
        String::from_utf8_lossy(&unknown.stderr),
    );

    let ci = read(".github/workflows/ci.yml");
    let rust_job = ci
        .split("  rust:")
        .nth(1)
        .expect("CI has Rust workspace job")
        .split("  public-contract-smoke:")
        .next()
        .expect("Rust workspace job is bounded");
    assert!(rust_job.contains("run: scripts/check.sh pr"));
    assert!(rust_job.contains("timeout-minutes: 30"));
    for duplicated_step in [
        "- name: Format",
        "- name: Clippy",
        "- name: Test",
        "- name: Build release",
        "- name: Verify deterministic screenshots",
        "- name: Release packaging check",
        "- name: Diff hygiene",
    ] {
        assert!(
            !rust_job.contains(duplicated_step),
            "Rust CI duplicates canonical check step: {duplicated_step}",
        );
    }
    for command in [
        "uvx \"zizmor@1.26.1\"",
        "cargo audit --deny warnings --ignore RUSTSEC-2024-0436",
        "cargo deny check licenses sources",
        "scripts/check-third-party-licenses.sh",
    ] {
        assert!(
            ci.contains(command),
            "CI release-policy parity omits {command}"
        );
        assert!(
            check.contains(command),
            "local release-policy parity omits {command}",
        );
    }
    assert!(!ci.contains("zizmor@1.26.1\"\n          --offline"));

    assert!(read("README.md").contains("scripts/check.sh fast"));
    assert!(read("README.md").contains("scripts/check.sh pr"));
    assert!(read("CONTRIBUTING.md").contains("scripts/check.sh pr"));
    assert!(read("docs/RELEASING.md").contains("scripts/check.sh release"));
}

#[test]
fn workspace_ci_bounds_heavy_rust_build_disk_usage() {
    let workflow = read(".github/workflows/ci.yml");

    assert!(workflow.contains("CARGO_INCREMENTAL: 0"));
    assert!(workflow.contains("CARGO_PROFILE_DEV_DEBUG: 0"));
    assert!(workflow.contains("CARGO_PROFILE_TEST_DEBUG: 0"));
    assert!(workflow.contains("- name: Cache cargo registry"));
    assert!(workflow.contains("key: cargo-registry-v1-${{ runner.os }}-"));
    assert!(!workflow.contains("- name: Cache cargo registry and build outputs\n        uses: actions/cache@v5\n        with:\n          path: |\n            ~/.cargo/registry\n            ~/.cargo/git\n            target\n          key: cargo-${{ runner.os }}-"));
}

#[test]
fn soak_tooling_is_bounded_fail_closed_and_documented() {
    let runner = read("scripts/run-supervised-soak.sh");
    assert!(runner.contains("--all-symbols"));
    assert!(runner.contains("--duration-secs"));
    assert!(runner.contains("--backfill-gaps"));
    assert!(runner.contains("--verify-parity"));
    assert!(runner.contains("kill -TERM"));
    assert!(runner.contains("report.json"));
    assert!(runner.contains("git_dirty"));
    assert!(runner.contains("binary_sha256"));
    assert!(runner.contains("runtime_source_sha256"));
    assert!(runner.contains("<data-dir>"));
    assert!(runner.contains("source tree or HEAD changed during soak evidence collection"));
    assert!(!runner.contains("--wallet"));
    assert!(!runner.contains("--private"));

    let validator = read("scripts/validate-soak-report.py");
    assert!(validator.contains("schema_version"));
    assert!(validator.contains("schema_version must equal 2"));
    assert!(validator.contains("git_dirty"));
    assert!(validator.contains("binary_sha256"));
    assert!(validator.contains("--binary"));
    assert!(validator.contains("clean_shutdown"));
    assert!(validator.contains("unrepaired_gaps"));
    assert!(validator.contains("parser_drops"));
    assert!(validator.contains("second_status"));

    let deployment = read("docs/deployment.md");
    assert!(deployment.contains("run-supervised-soak.sh"));
    assert!(deployment.contains("validate-soak-report.py"));
    assert!(deployment.contains("not multi-day soak proof"));
}

#[test]
fn public_docs_do_not_claim_unwired_runtime_features_and_list_local_writes() {
    let features = read("docs/feature-definitions.md");
    assert!(features.contains("not integrated into the production CLI or TUI"));

    let data_format = read("docs/data-format.md");
    assert!(data_format.contains("not loaded by the current CLI or TUI"));

    let privacy = read("docs/PRIVACY.md");
    for local_write in [
        "config.toml",
        "tui-preferences.toml",
        "alerts.jsonl",
        "Parquet",
        "schema manifest",
    ] {
        assert!(
            privacy.contains(local_write),
            "privacy documentation omits local write: {local_write}",
        );
    }

    let readme = read("README.md");
    assert!(readme.contains("Runtime commands currently use explicit CLI flags"));
}

#[test]
fn public_docs_state_identity_contribution_and_build_contracts() {
    let readme = read("README.md");
    let contributing = read("CONTRIBUTING.md");

    assert!(readme.contains("independent open-source project"));
    assert!(readme.contains("not affiliated with, endorsed by, or sponsored by Hyperliquid"));
    assert!(readme.contains("Developer ID Application"));
    assert!(readme.contains("not notarized"));
    assert!(readme.contains("does not claim notarization or Gatekeeper validation"));
    for prerequisite in [
        "Git",
        "Python 3",
        "rustup",
        "rustfmt",
        "clippy",
        "Xcode Command Line Tools",
        "build-essential",
        "pkg-config",
        "MSVC C++ Build Tools",
    ] {
        assert!(
            readme.contains(prerequisite),
            "README omits build prerequisite {prerequisite}",
        );
    }
    assert!(contributing.contains("licensed under the Apache"));
    assert!(contributing.contains("License, Version 2.0"));
    assert!(contributing.contains("No Contributor License Agreement (CLA) is required"));

    let license = read("LICENSE");
    let notice = read("NOTICE");
    let maintainers = read("MAINTAINERS.md");
    let cargo = read("Cargo.toml");
    assert!(license.contains("Apache License"));
    assert!(license.contains("Version 2.0, January 2004"));
    assert!(notice.contains("Copyright 2026 Rafal Sikora"));
    assert!(notice.contains("Maintained publicly by RSI Tech"));
    assert!(notice.contains("Website: https://rsitech.ai"));
    assert!(notice.contains("Contact: info@rsitech.ai"));
    assert!(maintainers.contains("publicly maintained by [RSI Tech]"));
    assert!(cargo.contains("authors = [\"RSI Tech <info@rsitech.ai>\"]"));
    assert!(cargo.contains("license = \"Apache-2.0\""));
    assert!(cargo.contains("homepage = \"https://rsitech.ai\""));
}

#[test]
fn public_routes_are_actionable_and_separate_security_conduct_and_questions() {
    let security = read("SECURITY.md");
    let conduct = read("CODE_OF_CONDUCT.md");
    let support = read("SUPPORT.md");
    let issue_config = read(".github/ISSUE_TEMPLATE/config.yml");

    assert!(security.contains("https://github.com/rsitech-ai/hlscreen/security/advisories/new"));
    assert!(security.contains("mailto:info@rsitech.ai?subject=hlscreen%20security%20report"));
    assert!(security.contains("acknowledge receipt within 3 business days"));
    assert!(security.contains("targets, not guarantees"));
    assert!(conduct.contains("mailto:info@rsitech.ai?subject=hlscreen%20conduct%20report"));
    assert!(conduct.contains("sole-maintainer project"));
    assert!(conduct.contains("independent"));
    assert!(conduct.contains("internal escalation channel"));
    assert!(conduct.contains("https://support.github.com/contact/report-abuse"));
    assert!(support.contains("https://github.com/rsitech-ai/hlscreen/discussions/categories/q-a"));
    assert!(support.contains("Reproducible defects belong in Issues"));
    assert!(issue_config.contains("name: Questions and support"));
    assert!(issue_config.starts_with("blank_issues_enabled: false\n"));

    let mut contacts = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_url: Option<String> = None;
    let mut current_about: Option<String> = None;
    for line in issue_config.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name: ") {
            if let Some(previous_name) = current_name.take() {
                contacts.push((
                    previous_name,
                    current_url.take().expect("contact link URL"),
                    current_about.take().expect("contact link description"),
                ));
            }
            current_name = Some(name.to_owned());
        } else if let Some(url) = trimmed.strip_prefix("url: ") {
            current_url = Some(url.to_owned());
        } else if let Some(about) = trimmed.strip_prefix("about: ") {
            current_about = Some(about.to_owned());
        }
    }
    if let Some(name) = current_name {
        contacts.push((
            name,
            current_url.expect("contact link URL"),
            current_about.expect("contact link description"),
        ));
    }
    assert_eq!(
        contacts.len(),
        2,
        "expected questions and security contacts"
    );
    assert!(contacts.iter().any(|(name, url, about)| {
        name == "Questions and support"
            && url == "https://github.com/rsitech-ai/hlscreen/discussions/categories/q-a"
            && about.contains("Discussions Q&A")
    }));
    assert!(contacts.iter().any(|(name, url, about)| {
        name == "Security issue"
            && url == "https://github.com/rsitech-ai/hlscreen/security/advisories/new"
            && about.contains("privately")
    }));
}

#[test]
fn public_docs_define_fixture_tooling_release_and_unreleased_contracts() {
    let fixtures = read("tests/fixtures/README.md");
    let data_format = read("docs/data-format.md");
    let docs_index = read("docs/README.md");
    let releasing = read("docs/RELEASING.md");
    let changelog = read("CHANGELOG.md");

    for classification in [
        "Synthetic and minimized fixtures",
        "Derived output fixtures",
        "Validation-report fixtures",
    ] {
        assert!(fixtures.contains(classification));
    }
    for entry in fs::read_dir(repo_root().join("tests/fixtures")).expect("fixture directory") {
        let entry = entry.expect("fixture directory entry");
        if entry.file_type().expect("fixture entry type").is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            assert!(
                fixtures.contains(&format!("`{name}/`")),
                "fixture directory is not classified: {name}",
            );
        }
    }
    for prohibited in [
        "credentials",
        "real accounts or wallets",
        "private streams",
        "unredacted user data",
    ] {
        assert!(fixtures.contains(prohibited));
    }
    assert!(data_format.contains("tests/fixtures/README.md"));
    assert!(docs_index.contains("2026-07-20"));
    assert!(
        releasing.contains("GitHub Releases is the only supported binary distribution channel")
    );
    assert!(releasing.contains("Do not redistribute workflow artifacts as releases"));
    assert!(changelog.contains("## 0.1.0 - 2026-07-20"));
    assert!(changelog.contains("## 0.1.1 - 2026-07-20"));
    assert!(changelog.contains("## 0.1.2 - 2026-07-21"));
    assert!(!changelog.contains("has not been published"));

    assert_relative_markdown_links_exist(&[
        "README.md",
        "SECURITY.md",
        "CODE_OF_CONDUCT.md",
        "SUPPORT.md",
        "CONTRIBUTING.md",
        "MAINTAINERS.md",
        "docs/README.md",
        "docs/data-format.md",
        "docs/RELEASING.md",
        "docs/releases/v0.1.0.md",
        "docs/releases/v0.1.1.md",
        "docs/releases/v0.1.2.md",
        "tests/fixtures/README.md",
    ]);
}

#[test]
fn public_readiness_gate_is_fail_closed_and_secret_safe() {
    let readiness = read("scripts/check-public-readiness.sh");

    for required in [
        "NOTICE",
        "MAINTAINERS.md",
        "THIRD_PARTY_LICENSES.txt",
        "THIRD_PARTY_NOTICES.md",
        "third_party/notices/manifest.json",
        "docs/releases/v0.1.0.md",
        "docs/releases/v0.1.1.md",
        "docs/releases/v0.1.2.md",
        "tests/fixtures/README.md",
        "scripts/check.sh",
        "scripts/check-history-secrets.sh",
        "scripts/summarize-git-identities.py",
        "scripts/summarize-git-history-privacy.py",
        "scripts/test-history-privacy.py",
    ] {
        assert!(
            readiness.contains(required),
            "public readiness omits required file {required}",
        );
    }
    for contract in [
        "placeholder_pattern",
        "obsolete_contact_pattern",
        "private_path_pattern",
        "credential_pattern",
        "unsafe_wording_pattern",
        "refs/tags/v0.1.0",
        "# hlscreen v0.1.0",
        "possible committed credential",
        "--redact=100",
        "--log-opts=\\\"--all\\\"",
    ] {
        assert!(
            readiness.contains(contract),
            "public readiness omits fail-closed contract {contract}",
        );
    }
    assert!(!readiness.contains("cat \"$scan_dir/credentials"));
    assert!(!readiness.contains("cat \"$scan_dir/history-credentials"));
    assert!(!readiness.contains("history.patch"));
}

#[test]
fn history_secret_scan_pins_gitleaks_and_redacts_findings() {
    let scanner = read("scripts/check-history-secrets.sh");

    for contract in [
        "8.30.1",
        "gitleaks git",
        "--redact=100",
        "refs/scan/public-readiness",
        "scan_id",
        "$scan_namespace/heads",
        "$scan_namespace/pulls",
        "+refs/heads/*",
        "+refs/pull/*/head",
        "--all",
        "RuleID",
        "Commit",
        "File",
        "StartLine",
        "path_sha256",
        "ref_count",
        "%ae%x09%ce",
        "identity_summary",
        "privacy_summary",
        "summarize-git-history-privacy.py",
    ] {
        assert!(
            scanner.contains(contract),
            "history scanner omits {contract}"
        );
    }
    assert!(!scanner.contains("Match\""));
    assert!(!scanner.contains("Secret\""));
    assert!(!scanner.contains("path={path}"));
    assert!(!scanner.contains("ref={refs}"));

    let privacy_mock = Command::new("python3")
        .arg("scripts/test-history-privacy.py")
        .current_dir(repo_root())
        .output()
        .expect("run history privacy metadata test");
    assert!(
        privacy_mock.status.success(),
        "history privacy test failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&privacy_mock.stdout),
        String::from_utf8_lossy(&privacy_mock.stderr),
    );
    assert!(
        String::from_utf8_lossy(&privacy_mock.stdout)
            .contains("history_privacy_mock_tests=passed cases=1")
    );

    let mut identity_check = Command::new("python3")
        .arg("scripts/summarize-git-identities.py")
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run identity summarizer");
    identity_check
        .stdin
        .take()
        .expect("identity summarizer stdin")
        .write_all(
            b"12345+bot@users.noreply.github.com\t12345+bot@users.noreply.github.com\n\
person@example.test\tperson@example.test\n",
        )
        .expect("write identity fixtures");
    let identity_output = identity_check
        .wait_with_output()
        .expect("collect identity summary");
    assert!(identity_output.status.success());
    let identity_stdout = String::from_utf8_lossy(&identity_output.stdout);
    assert!(identity_stdout.contains("identity_metadata=commits:2"));
    assert!(identity_stdout.contains("author_non_noreply_occurrences:1"));
    assert!(identity_stdout.contains("committer_non_noreply_occurrences:1"));
    assert!(identity_stdout.contains("unique_non_noreply_mailboxes:1"));
    assert!(!identity_stdout.contains('@'));

    let embedded_python = scanner
        .split("<<'PY'\n")
        .nth(1)
        .and_then(|source| source.rsplit_once("\nPY").map(|(source, _)| source))
        .expect("history scanner has embedded Python");
    let syntax = Command::new("python3")
        .args([
            "-c",
            "import sys; compile(sys.argv[1], '<gitleaks-summary>', 'exec')",
            embedded_python,
        ])
        .output()
        .expect("compile embedded gitleaks summary Python");
    assert!(
        syntax.status.success(),
        "embedded gitleaks summary Python is invalid: {}",
        String::from_utf8_lossy(&syntax.stderr),
    );

    let temp = tempfile::tempdir().expect("temp dir");
    let fake = temp.path().join("gitleaks");
    fs::write(
        &fake,
        "#!/usr/bin/env bash\nif [[ ${1:-} == version ]]; then echo 8.30.0; exit 0; fi\nexit 99\n",
    )
    .expect("write fake gitleaks");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&fake).expect("fake metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake, permissions).expect("make fake executable");
    }
    let path = format!(
        "{}:{}",
        temp.path().display(),
        env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("bash")
        .arg("scripts/check-history-secrets.sh")
        .env("PATH", path)
        .current_dir(repo_root())
        .output()
        .expect("run history scanner with wrong version");
    assert!(!output.status.success(), "wrong gitleaks version passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exactly 8.30.1"),
        "unexpected error: {stderr}"
    );
}
