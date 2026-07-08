use std::{env, fs, path::PathBuf};

fn repo_root() -> PathBuf {
    env::var("HLS_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("current dir"))
}

fn read(path: &str) -> String {
    let full_path = repo_root().join(path);
    fs::read_to_string(&full_path).unwrap_or_else(|err| {
        panic!("read {}: {err}", full_path.display());
    })
}

#[test]
fn dist_workspace_declares_tag_gated_release_plan() {
    let dist = read("dist-workspace.toml");

    assert!(dist.contains("[dist]"));
    assert!(dist.contains("ci = \"github\""));
    assert!(dist.contains("pr-run-mode = \"plan\""));
    assert!(dist.contains("hosting = \"github\""));
    assert!(dist.contains("github-attestations = true"));
    assert!(dist.contains("install-updater = false"));
    assert!(dist.contains("homebrew"));
}

#[test]
fn release_workflow_is_pull_request_plan_and_tag_publish_only() {
    let workflow = read(".github/workflows/release.yml");

    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("v*"));
    assert!(workflow.contains("run: dist plan"));
    assert!(workflow.contains("run: dist build"));
    assert!(!workflow.contains("cargo dist plan"));
    assert!(!workflow.contains("cargo dist build"));
    assert!(!workflow.contains("pull_request_target"));
    assert!(!workflow.contains("${{ secrets."));
}

#[test]
fn distributable_crate_inherits_public_repository_metadata() {
    let manifest = read("crates/hls-cli/Cargo.toml");

    assert!(manifest.contains("repository.workspace = true"));
    assert!(manifest.contains("homepage.workspace = true"));
    assert!(manifest.contains("description.workspace = true"));
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
