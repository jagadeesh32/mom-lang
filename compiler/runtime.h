#ifndef MOM_RUNTIME_H
#define MOM_RUNTIME_H

#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

// ── Value tags ────────────────────────────────────────────────────────────────
#define MOM_TAG_INT     0
#define MOM_TAG_FLOAT   1
#define MOM_TAG_BOOL    2
#define MOM_TAG_STRING  3
#define MOM_TAG_LIST    4
#define MOM_TAG_VARIANT 5   // enum variant
#define MOM_TAG_STRUCT  6
#define MOM_TAG_UNIT    7

// ── Core value type ───────────────────────────────────────────────────────────
typedef struct MomVal MomVal;

struct MomVal {
    int tag;
    union {
        int64_t   i;     // INT
        double    f;     // FLOAT
        int       b;     // BOOL (0/1)
        char     *s;     // STRING (heap-allocated, NUL-terminated)
        struct {         // LIST
            MomVal **items;
            int len;
            int cap;
        } list;
        struct {         // VARIANT (enum)
            int      variant_id;
            MomVal **payload;
            int      payload_len;
        } variant;
        struct {         // STRUCT
            int      type_id;
            MomVal **fields;
            int      field_count;
            const char **field_names;
        } strct;
    } data;
};

// ── Constructors ──────────────────────────────────────────────────────────────
MomVal *mom_int(int64_t n);
MomVal *mom_float(double f);
MomVal *mom_bool(int b);
MomVal *mom_str(const char *s);
MomVal *mom_str_owned(char *s);   // takes ownership
MomVal *mom_unit(void);

// List
MomVal *mom_list_new(void);
void    mom_list_push(MomVal *list, MomVal *item);
MomVal *mom_list_from_n(int n, ...);  // mom_list_from_n(3, a, b, c)

// Enum variant
MomVal *mom_variant(int tag, int payload_len, ...); // variadic payload

// Struct
MomVal *mom_struct(int type_id, int n, ...);  // n pairs of (name, value)

// ── Accessors ─────────────────────────────────────────────────────────────────
int64_t    mom_int_val(MomVal *v);
double     mom_float_val(MomVal *v);
int        mom_bool_val(MomVal *v);
const char *mom_str_val(MomVal *v);

// List ops
int     mom_list_len(MomVal *v);
MomVal *mom_list_get(MomVal *v, int i);
void    mom_list_set(MomVal *v, int i, MomVal *item);
MomVal *mom_list_pop(MomVal *v);
void    mom_list_insert(MomVal *v, int i, MomVal *item);
void    mom_list_remove(MomVal *v, int i);
MomVal *mom_list_concat(MomVal *a, MomVal *b);

// Variant (enum) ops
int     mom_variant_tag(MomVal *v);
MomVal *mom_variant_payload(MomVal *v, int i);

// Struct field ops
MomVal *mom_struct_get(MomVal *v, int field_idx);
void    mom_struct_set(MomVal *v, int field_idx, MomVal *item);

// ── String operations ─────────────────────────────────────────────────────────
MomVal *mom_str_concat(MomVal *a, MomVal *b);
MomVal *mom_str_concat_c(MomVal *a, const char *b);
int     mom_str_len(MomVal *v);
MomVal *mom_str_char_at(MomVal *v, int i);
int     mom_str_eq(MomVal *a, MomVal *b);
int     mom_str_contains(MomVal *haystack, MomVal *needle);
int     mom_str_starts_with(MomVal *s, MomVal *prefix);
int     mom_str_ends_with(MomVal *s, MomVal *suffix);
MomVal *mom_str_slice(MomVal *s, int start, int end);
MomVal *mom_str_replace(MomVal *s, MomVal *from, MomVal *to);
MomVal *mom_str_split(MomVal *s, MomVal *delim);
MomVal *mom_str_strip(MomVal *s);
MomVal *mom_str_upper(MomVal *s);
MomVal *mom_str_lower(MomVal *s);
MomVal *mom_int_to_str(MomVal *v);
MomVal *mom_float_to_str(MomVal *v);
MomVal *mom_bool_to_str(MomVal *v);

// ── I/O ───────────────────────────────────────────────────────────────────────
void    mom_print(MomVal *v);
void    mom_println(MomVal *v);
void    mom_eprint(MomVal *v);
MomVal *mom_input(MomVal *prompt);
MomVal *mom_read_file(MomVal *path);
void    mom_write_file(MomVal *path, MomVal *content);
MomVal *mom_getenv(MomVal *name);

// ── Arithmetic helpers ────────────────────────────────────────────────────────
MomVal *mom_add(MomVal *a, MomVal *b);
MomVal *mom_sub(MomVal *a, MomVal *b);
MomVal *mom_mul(MomVal *a, MomVal *b);
MomVal *mom_div(MomVal *a, MomVal *b);
MomVal *mom_mod(MomVal *a, MomVal *b);
MomVal *mom_neg(MomVal *a);
MomVal *mom_not(MomVal *a);
MomVal *mom_eq(MomVal *a, MomVal *b);
MomVal *mom_ne(MomVal *a, MomVal *b);
MomVal *mom_lt(MomVal *a, MomVal *b);
MomVal *mom_le(MomVal *a, MomVal *b);
MomVal *mom_gt(MomVal *a, MomVal *b);
MomVal *mom_ge(MomVal *a, MomVal *b);

// ── Conversion ────────────────────────────────────────────────────────────────
MomVal *mom_to_str(MomVal *v);        // any value to String
MomVal *mom_to_int(MomVal *v);        // String/Float/Bool to Int
MomVal *mom_to_float(MomVal *v);      // String/Int/Bool to Float
MomVal *mom_to_bool(MomVal *v);       // any to Bool

// Type check
int mom_is_int(MomVal *v);
int mom_is_float(MomVal *v);
int mom_is_bool(MomVal *v);
int mom_is_str(MomVal *v);
int mom_is_list(MomVal *v);
int mom_is_unit(MomVal *v);

// ── Control ───────────────────────────────────────────────────────────────────
void    mom_panic(const char *msg);
MomVal *mom_range(int64_t n);
MomVal *mom_range2(int64_t start, int64_t end);
MomVal *mom_range3(int64_t start, int64_t end, int64_t step);

// ── Memory ────────────────────────────────────────────────────────────────────
// Simple arena - no GC in stage-1.
void *mom_alloc(size_t size);
char *mom_strdup(const char *s);
char *mom_strcat_alloc(const char *a, const char *b);

// ── Stage-1 native print helpers ──────────────────────────────────────────────
// These take raw C types, not MomVal*, for use by stage-1 compiled programs.
#include <inttypes.h>
void mom_print_int(int64_t n);
void mom_print_bool(int b);
void mom_print_unit(void);

// ── Entry point ───────────────────────────────────────────────────────────────
// stage-1 compiled programs define mom_main(); runtime provides main().
void mom_main(void);

#endif // MOM_RUNTIME_H
