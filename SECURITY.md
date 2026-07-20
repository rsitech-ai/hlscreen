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

Use [GitHub private vulnerability reporting](https://github.com/rsitech-ai/hlscreen/security/advisories/new) as the primary route. If that form is unavailable, email [info@rsitech.ai with the subject `hlscreen security report`](mailto:info@rsitech.ai?subject=hlscreen%20security%20report). Do not include credentials, private keys, or unnecessary personal data.

Include:

- A concise description of the issue.
- Reproduction steps.
- Affected commit or version.
- Any logs or payloads needed to reproduce, with secrets removed.
- Your assessment of impact.

The maintainers aim to acknowledge receipt within 3 business days and provide
an initial assessment within 10 business days. These are response targets, not guarantees; remediation and disclosure timing depend on the issue's scope and impact.

## Safety Boundary

`hlscreen` must remain read-only market-data infrastructure. A change that introduces signing, wallet permissions, order placement, withdrawals, private account streams, or execution controls should be treated as a security issue unless it is part of an explicitly approved future design that changes the project scope.
