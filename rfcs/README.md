# mom RFCs

Non-trivial language and tooling changes go through an RFC. Trivial
changes — bug fixes, internal refactors, stdlib additions that don't
change the surface — ship as normal PRs.

## When you need an RFC

- New surface syntax or a change to existing syntax.
- A change in the borrow checker, type system, or unsafe semantics.
- A change in the binary layout, calling convention, or FFI shape.
- A change in `mom.toml`, the CLI surface, or the package format.
- Anything that breaks existing user code that compiles today.

## How to file one

1. Copy `0000-template.md` to `XXXX-short-name.md` where `XXXX` is the
   next free number.
2. Fill in every section. "N/A" is fine where it genuinely is, but
   "TBD" is not.
3. Open a PR titled `rfc: <short name>` and tag the maintainers in the
   relevant phase.
4. Discussion happens in the PR. Substantive points are folded into
   the RFC text — *the RFC, not the thread, is the eventual record*.

## States

- `draft` — actively being written.
- `accepted` — merged on `main`; tracked as an issue until implemented.
- `rejected` — closed PR; the rejection rationale is preserved in the
  RFC body so future authors can find it.
- `superseded` — replaced by a later RFC; both link to each other.

## Numbering

RFC numbers are dense and never reused. If RFC #0042 is rejected, the
file stays at `0042-…` with `status: rejected`.
