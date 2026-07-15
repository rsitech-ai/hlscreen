use std::{env, fs, path::PathBuf, process::Command};

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
fn vendored_spec_kit_has_source_version_scope_and_complete_license() {
    let notice = read("THIRD_PARTY_NOTICES.md");
    let license = read("third_party/spec-kit/LICENSE");
    let integration = read(".specify/integration.json");
    let init_options = read(".specify/init-options.json");
    let manifest = read(".specify/integrations/speckit.manifest.json");
    let notice_manifest = read("third_party/notices/manifest.json");
    let parquet_notice = read("third_party/notices/parquet-59.1.0-NOTICE.txt");
    let notice_checker = read("scripts/check-third-party-notices.py");

    assert!(integration.contains("\"version\": \"0.11.1\""));
    assert!(init_options.contains("\"speckit_version\": \"0.11.1\""));
    assert!(manifest.contains("\"version\": \"0.11.1\""));
    assert!(notice.contains("Spec Kit 0.11.1"));
    assert!(notice.contains("https://github.com/github/spec-kit/tree/v0.11.1"));
    assert!(notice.contains("`.specify/`"));
    assert!(notice.contains("`.agents/skills/speckit-*/`"));
    assert!(notice.contains("Copyright GitHub, Inc."));
    assert!(notice.contains("third_party/spec-kit/LICENSE"));
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
    assert!(license.contains("Copyright GitHub, Inc."));
    assert!(license.contains("Permission is hereby granted, free of charge"));
    assert!(license.contains("THE SOFTWARE IS PROVIDED \"AS IS\""));
    assert!(license.contains("LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE"));
}

#[test]
fn local_release_archive_stages_project_and_third_party_notices() {
    let local_smoke = read("scripts/local-release-artifact-smoke.sh");
    let releasing = read("docs/RELEASING.md");

    for file in [
        "LICENSE",
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
    assert!(release.contains("cargo-auditable/releases/download/v0.7.5"));
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
    assert!(public_scan.contains("docs/evidence/soak/sota-allpairs-20260713-15m.json"));
    assert!(public_scan.contains("scripts/harden-generated-release-workflow.py"));
    assert!(public_scan.contains("Release tag created"));
    assert!(public_scan.contains("private_path_pattern"));
    assert!(public_scan.contains("credential_pattern"));
    assert!(public_scan.matches("git grep -n -E -e").count() >= 2);
    assert!(public_scan.contains("git log -p --all --no-ext-diff --no-textconv"));
    assert!(public_scan.contains("credential_status > 1"));
    assert!(public_scan.contains("history_credential_status > 1"));

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
    assert!(packaging_check.contains("sota-allpairs-20260713-15m.json"));
    assert!(packaging_check.contains("merge-base --is-ancestor"));
    assert!(packaging_check.contains("soak-report-invalid.json"));
    assert!(packaging_check.contains("soak-report-invalid-command.json"));
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
    assert!(!runner.contains("--wallet"));
    assert!(!runner.contains("--private"));

    let validator = read("scripts/validate-soak-report.py");
    assert!(validator.contains("schema_version"));
    assert!(validator.contains("clean_shutdown"));
    assert!(validator.contains("unrepaired_gaps"));
    assert!(validator.contains("parser_drops"));
    assert!(validator.contains("second_status"));

    let deployment = read("docs/deployment.md");
    assert!(deployment.contains("run-supervised-soak.sh"));
    assert!(deployment.contains("validate-soak-report.py"));
    assert!(deployment.contains("not multi-day soak proof"));
}
