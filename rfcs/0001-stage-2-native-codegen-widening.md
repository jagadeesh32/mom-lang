---
title: "Stage-2 native codegen for String, List, Struct, Enum, Match"
authors: ["mom-core"]
status: "draft"
phase: "1.2"
discussion: "TBD"
implementation: "TBD"
---

# Summary

Widen the native codegen-to-C path beyond the Int/Bool/Unit/Float
subset to cover String, List, Struct, Enum, and Match. Each new type
needs runtime support (allocator + lifecycle) and a representation
decision that the borrow checker can already reason about.

# Motivation

The interpreter handles all surface syntax today. The native backend
only handles arithmetic-style programs. To use mom for the
infrastructure use cases in `docs/philosophy.md`, we need to compile
programs that use heap-allocated values: web handlers, parsers, log
processors, anything that touches a string or a collection.

# Detailed design

The work splits into five RFC-tracked deliverables, each with its own
acceptance test in `tests/native_build.rs`:

| Deliverable | Runtime surface needed                              | Acceptance test                            |
|-------------|------------------------------------------------------|--------------------------------------------|
| `String`    | `mom_string` heap type with refcount; `mom_print_str`, `mom_string_concat`, `mom_string_eq` | `native_string_concat_and_print`           |
| `List`      | `mom_list` of `mom_value` with grow-on-push          | `native_list_len_index_push_iterate`       |
| `Struct`    | One emitted `struct` per `StructDecl`; field reads as direct member access | `native_struct_declaration_and_field_access` |
| `Enum`      | Tagged-union `struct { uint32_t tag; union { … } payload; }` | `native_enum_variant_round_trip`           |
| `Match`     | Switch on `tag`; pattern bindings via assignment from `payload.<arm>` | `native_match_dispatches_on_variant`       |

Codegen lands in dependency order: String → List → Struct → Enum →
Match. Each step extends `CType`, adds the C-side helpers in
`runtime/runtime.{h,c}`, and grows the borrow-checker integration so
the Move/Copy classification keeps working.

# Drawbacks

- Increases the runtime's surface and binary size. Mitigation: each
  helper is opt-in — programs that don't use strings won't pull in
  the string runtime.
- The naive String design (Rc + concat-copy) is not the design we'd
  pick for production. Phase 1.3 replaces it with a `Vec<u8>` builder
  and SSO inline strings.

# Rationale and alternatives

- LLVM IR direct emission: bigger investment, but unlocks SIMD and
  better inlining. Out of scope here; the C path is the bootstrap
  contract.
- Skip String/List, ship only Struct/Enum/Match: leaves the language
  not-useful for stdlib needs (`std::fmt`, `std::serde`).

# Acceptance criteria

Each of the 5 acceptance tests passes; the `mom run examples/*.mom`
matrix continues to match interpreter output for every example that
uses the new types.
