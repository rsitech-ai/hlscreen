# Development Tooling Provenance

The pinned Spec Kit material under `.specify/` and
`.agents/skills/speckit-*/` is developer-only workflow tooling. It is not
compiled into `hls`, loaded by the runtime, or required to use a release
binary. The vendored version is Spec Kit 0.11.1; its source, scope, and license
are recorded in `THIRD_PARTY_NOTICES.md` and `third_party/spec-kit/LICENSE`.

The JSON manifests under `.specify/integrations/` are integrity inventories for
the installed integration files. They help reviewers detect local drift, but
they are not signatures and do not by themselves establish that a file is safe
or current upstream content.

Project-authored requirements and decisions live under `specs/` and in
`.specify/memory/constitution.md`; `.specify/feature.json` selects this
repository's active feature. Those files are project-authored content, not part
of the upstream Spec Kit distribution. Local project configuration under
`.specify/extensions/agent-context/agent-context-config.yml` is also reviewed as
repository configuration.

## Update Policy

Spec Kit changes require a reviewed, pinned update to an explicit upstream
version. An update must:

1. Verify the upstream tag, source URL, and license before copying files.
2. Review the complete vendored diff, including generated manifests and skills.
3. Inspect every changed executable script. This includes both Shell and PowerShell
   entrypoints when either is present; generated code is not trusted
   merely because it came from a tool.
4. Refresh `THIRD_PARTY_NOTICES.md`, the vendored license, version inventories,
   and integrity hashes in the same change.
5. Run the release validation gate and keep the runtime/package boundary free
   of developer-only tooling.

Do not run newly updated scripts before that review is complete.
