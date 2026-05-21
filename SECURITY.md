# Security Policy

## Supported versions

Until `mom 1.0` ships, only the **latest** tagged release is supported.
Security backports to older tags are best-effort; we recommend
upgrading on every release.

| Version | Supported          |
|---------|--------------------|
| `0.6.x` | ✅ current          |
| `0.5.x` | 🟡 critical fixes only |
| `< 0.5` | ❌ unsupported      |

When `mom 1.0` lands (Phase 7 milestone), the support policy becomes
`current` + `current - 1`, in line with the rest of the systems-language
ecosystem.

## Reporting a vulnerability

Please **do not** open a public issue for suspected vulnerabilities.
Email the maintainers at **security@mom-lang.dev** with:

- A description of the issue.
- A reproducer (preferably a `.mom` file plus the exact `mom`
  subcommand that triggers it).
- The output of `mom version`.
- Your assessment of impact: confidentiality / integrity / availability,
  and whether user-supplied input is required.

We will acknowledge receipt within **2 business days** and aim to ship
a patched release within **30 days** for critical issues. You will be
credited in the release notes unless you ask to remain anonymous.

## Coordinated disclosure

Once a fix is staged we will:

1. Privately confirm the patch with the reporter.
2. Cut a patch release on the affected branch(es).
3. Publish a `GHSA-####-####-####` advisory in the GitHub Security
   Advisories tab linking the commit and the upgrade path.
4. Add a row to `docs/risks.md` under "Known historical issues".

## Out of scope

- Compiler crashes on syntactically invalid input that produce a
  well-formed diagnostic (these are bugs, but file them as regular
  issues).
- Performance regressions without an integrity or confidentiality
  impact.
- Issues affecting third-party tools that consume mom's JSON / DAP
  output — report those to the relevant project.
