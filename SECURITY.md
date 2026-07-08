# Security Policy

## Supported Versions

The `main` branch is the only supported branch before the first stable release.

## Scope

Security-sensitive areas include:

- Read-only API boundaries.
- Hyperliquid public API parsing and validation.
- Local recorder paths and file handling.
- SQLite metadata registry behavior.
- Dependency and supply-chain issues.
- Any accidental wallet, key, private-user, order, cancel, withdrawal, leverage, or exchange-action surface.

## Reporting A Vulnerability

Do not open a public issue for security vulnerabilities.

After the repository is public, use GitHub Security Advisories if available. If advisories are not available yet, contact the maintainer privately through GitHub and include:

- A concise description of the issue.
- Reproduction steps.
- Affected commit or version.
- Any logs or payloads needed to reproduce, with secrets removed.
- Your assessment of impact.

## Safety Boundary

`hlscreen` must remain read-only market-data infrastructure. A change that introduces signing, wallet permissions, order placement, withdrawals, private account streams, or execution controls should be treated as a security issue unless it is part of an explicitly approved future design that changes the project scope.
