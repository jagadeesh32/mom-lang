/*
 * mom runtime — stage-1 C runtime library.
 *
 * Implements all functions declared in runtime.h.
 * Compiled with: gcc -std=c99 -c runtime.c
 */

#include "runtime.h"

#include <stdarg.h>
#include <ctype.h>
#include <inttypes.h>
#include <errno.h>

// ── Memory ────────────────────────────────────────────────────────────────────

void *mom_alloc(size_t size) {
    void *p = malloc(size);
    if (!p) {
        fputs("mom: out of memory\n", stderr);
        exit(1);
    }
    return p;
}

char *mom_strdup(const char *s) {
    size_t len = strlen(s);
    char *copy = (char *)mom_alloc(len + 1);
    memcpy(copy, s, len + 1);
    return copy;
}

char *mom_strcat_alloc(const char *a, const char *b) {
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char *out = (char *)mom_alloc(la + lb + 1);
    memcpy(out, a, la);
    memcpy(out + la, b, lb + 1);
    return out;
}

// ── Control ───────────────────────────────────────────────────────────────────

void mom_panic(const char *msg) {
    fprintf(stderr, "mom panic: %s\n", msg);
    exit(1);
}

// ── Constructors ──────────────────────────────────────────────────────────────

MomVal *mom_int(int64_t n) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_INT;
    v->data.i = n;
    return v;
}

MomVal *mom_float(double f) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_FLOAT;
    v->data.f = f;
    return v;
}

MomVal *mom_bool(int b) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_BOOL;
    v->data.b = b ? 1 : 0;
    return v;
}

MomVal *mom_str(const char *s) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_STRING;
    v->data.s = mom_strdup(s);
    return v;
}

MomVal *mom_str_owned(char *s) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_STRING;
    v->data.s = s;
    return v;
}

MomVal *mom_unit(void) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_UNIT;
    v->data.i = 0;
    return v;
}

// List

MomVal *mom_list_new(void) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_LIST;
    v->data.list.len = 0;
    v->data.list.cap = 0;
    v->data.list.items = NULL;
    return v;
}

void mom_list_push(MomVal *list, MomVal *item) {
    if (list->tag != MOM_TAG_LIST) mom_panic("mom_list_push: not a list");
    if (list->data.list.len >= list->data.list.cap) {
        int new_cap = list->data.list.cap == 0 ? 8 : list->data.list.cap * 2;
        MomVal **new_items = (MomVal **)mom_alloc((size_t)new_cap * sizeof(MomVal *));
        if (list->data.list.items) {
            memcpy(new_items, list->data.list.items,
                   (size_t)list->data.list.len * sizeof(MomVal *));
            free(list->data.list.items);
        }
        list->data.list.items = new_items;
        list->data.list.cap = new_cap;
    }
    list->data.list.items[list->data.list.len++] = item;
}

MomVal *mom_list_from_n(int n, ...) {
    MomVal *list = mom_list_new();
    va_list ap;
    va_start(ap, n);
    for (int i = 0; i < n; i++) {
        MomVal *item = va_arg(ap, MomVal *);
        mom_list_push(list, item);
    }
    va_end(ap);
    return list;
}

// Enum variant

MomVal *mom_variant(int tag, int payload_len, ...) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_VARIANT;
    v->data.variant.variant_id = tag;
    v->data.variant.payload_len = payload_len;
    if (payload_len > 0) {
        v->data.variant.payload = (MomVal **)mom_alloc(
            (size_t)payload_len * sizeof(MomVal *));
        va_list ap;
        va_start(ap, payload_len);
        for (int i = 0; i < payload_len; i++) {
            v->data.variant.payload[i] = va_arg(ap, MomVal *);
        }
        va_end(ap);
    } else {
        v->data.variant.payload = NULL;
    }
    return v;
}

// Struct: n pairs of (const char *name, MomVal *value)

MomVal *mom_struct(int type_id, int n, ...) {
    MomVal *v = (MomVal *)mom_alloc(sizeof(MomVal));
    v->tag = MOM_TAG_STRUCT;
    v->data.strct.type_id = type_id;
    v->data.strct.field_count = n;
    if (n > 0) {
        v->data.strct.fields = (MomVal **)mom_alloc(
            (size_t)n * sizeof(MomVal *));
        v->data.strct.field_names = (const char **)mom_alloc(
            (size_t)n * sizeof(const char *));
        va_list ap;
        va_start(ap, n);
        for (int i = 0; i < n; i++) {
            const char *name = va_arg(ap, const char *);
            MomVal *val      = va_arg(ap, MomVal *);
            v->data.strct.field_names[i] = name;
            v->data.strct.fields[i]      = val;
        }
        va_end(ap);
    } else {
        v->data.strct.fields      = NULL;
        v->data.strct.field_names = NULL;
    }
    return v;
}

// ── Accessors ─────────────────────────────────────────────────────────────────

int64_t mom_int_val(MomVal *v) {
    if (v->tag != MOM_TAG_INT) mom_panic("mom_int_val: not an int");
    return v->data.i;
}

double mom_float_val(MomVal *v) {
    if (v->tag != MOM_TAG_FLOAT) mom_panic("mom_float_val: not a float");
    return v->data.f;
}

int mom_bool_val(MomVal *v) {
    if (v->tag != MOM_TAG_BOOL) mom_panic("mom_bool_val: not a bool");
    return v->data.b;
}

const char *mom_str_val(MomVal *v) {
    if (v->tag != MOM_TAG_STRING) mom_panic("mom_str_val: not a string");
    return v->data.s;
}

// List ops

int mom_list_len(MomVal *v) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_len: not a list");
    return v->data.list.len;
}

MomVal *mom_list_get(MomVal *v, int i) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_get: not a list");
    if (i < 0 || i >= v->data.list.len) mom_panic("mom_list_get: index out of bounds");
    return v->data.list.items[i];
}

void mom_list_set(MomVal *v, int i, MomVal *item) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_set: not a list");
    if (i < 0 || i >= v->data.list.len) mom_panic("mom_list_set: index out of bounds");
    v->data.list.items[i] = item;
}

MomVal *mom_list_pop(MomVal *v) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_pop: not a list");
    if (v->data.list.len == 0) mom_panic("mom_list_pop: empty list");
    return v->data.list.items[--v->data.list.len];
}

void mom_list_insert(MomVal *v, int i, MomVal *item) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_insert: not a list");
    if (i < 0 || i > v->data.list.len) mom_panic("mom_list_insert: index out of bounds");
    /* grow first */
    mom_list_push(v, item); /* ensures capacity */
    /* shift right */
    for (int j = v->data.list.len - 1; j > i; j--) {
        v->data.list.items[j] = v->data.list.items[j - 1];
    }
    v->data.list.items[i] = item;
}

void mom_list_remove(MomVal *v, int i) {
    if (v->tag != MOM_TAG_LIST) mom_panic("mom_list_remove: not a list");
    if (i < 0 || i >= v->data.list.len) mom_panic("mom_list_remove: index out of bounds");
    for (int j = i; j < v->data.list.len - 1; j++) {
        v->data.list.items[j] = v->data.list.items[j + 1];
    }
    v->data.list.len--;
}

MomVal *mom_list_concat(MomVal *a, MomVal *b) {
    if (a->tag != MOM_TAG_LIST) mom_panic("mom_list_concat: a not a list");
    if (b->tag != MOM_TAG_LIST) mom_panic("mom_list_concat: b not a list");
    MomVal *out = mom_list_new();
    for (int i = 0; i < a->data.list.len; i++)
        mom_list_push(out, a->data.list.items[i]);
    for (int i = 0; i < b->data.list.len; i++)
        mom_list_push(out, b->data.list.items[i]);
    return out;
}

// Variant ops

int mom_variant_tag(MomVal *v) {
    if (v->tag != MOM_TAG_VARIANT) mom_panic("mom_variant_tag: not a variant");
    return v->data.variant.variant_id;
}

MomVal *mom_variant_payload(MomVal *v, int i) {
    if (v->tag != MOM_TAG_VARIANT) mom_panic("mom_variant_payload: not a variant");
    if (i < 0 || i >= v->data.variant.payload_len)
        mom_panic("mom_variant_payload: index out of bounds");
    return v->data.variant.payload[i];
}

// Struct field ops

MomVal *mom_struct_get(MomVal *v, int field_idx) {
    if (v->tag != MOM_TAG_STRUCT) mom_panic("mom_struct_get: not a struct");
    if (field_idx < 0 || field_idx >= v->data.strct.field_count)
        mom_panic("mom_struct_get: field index out of bounds");
    return v->data.strct.fields[field_idx];
}

void mom_struct_set(MomVal *v, int field_idx, MomVal *item) {
    if (v->tag != MOM_TAG_STRUCT) mom_panic("mom_struct_set: not a struct");
    if (field_idx < 0 || field_idx >= v->data.strct.field_count)
        mom_panic("mom_struct_set: field index out of bounds");
    v->data.strct.fields[field_idx] = item;
}

// ── Type checks ───────────────────────────────────────────────────────────────

int mom_is_int(MomVal *v)   { return v->tag == MOM_TAG_INT; }
int mom_is_float(MomVal *v) { return v->tag == MOM_TAG_FLOAT; }
int mom_is_bool(MomVal *v)  { return v->tag == MOM_TAG_BOOL; }
int mom_is_str(MomVal *v)   { return v->tag == MOM_TAG_STRING; }
int mom_is_list(MomVal *v)  { return v->tag == MOM_TAG_LIST; }
int mom_is_unit(MomVal *v)  { return v->tag == MOM_TAG_UNIT; }

// ── String representation (internal, returns heap-allocated C string) ─────────

static char *mom_val_to_cstr(MomVal *v);

static char *mom_list_to_cstr(MomVal *v) {
    /* "[a, b, c]" */
    size_t cap = 64;
    char *buf = (char *)mom_alloc(cap);
    buf[0] = '[';
    size_t pos = 1;

    for (int i = 0; i < v->data.list.len; i++) {
        char *s = mom_val_to_cstr(v->data.list.items[i]);
        size_t slen = strlen(s);
        /* ", " separator */
        size_t need = pos + slen + 3; /* ", " + s + "]" + NUL */
        if (need > cap) {
            while (cap < need) cap *= 2;
            char *nb = (char *)mom_alloc(cap);
            memcpy(nb, buf, pos);
            free(buf);
            buf = nb;
        }
        if (i > 0) { buf[pos++] = ','; buf[pos++] = ' '; }
        memcpy(buf + pos, s, slen);
        pos += slen;
        free(s);
    }
    buf[pos++] = ']';
    buf[pos] = '\0';
    return buf;
}

static char *mom_variant_to_cstr(MomVal *v) {
    /* "Variant" or "Variant(a, b)" — we don't have names at runtime,
     * so we render the numeric id. */
    char id_buf[32];
    snprintf(id_buf, sizeof(id_buf), "Variant%d", v->data.variant.variant_id);
    if (v->data.variant.payload_len == 0) {
        return mom_strdup(id_buf);
    }
    /* build "Variant(a, b, ...)" */
    size_t cap = 64;
    char *buf = (char *)mom_alloc(cap);
    size_t id_len = strlen(id_buf);
    if (id_len + 2 > cap) { cap = id_len + 64; free(buf); buf = (char *)mom_alloc(cap); }
    memcpy(buf, id_buf, id_len);
    size_t pos = id_len;
    buf[pos++] = '(';
    for (int i = 0; i < v->data.variant.payload_len; i++) {
        char *s = mom_val_to_cstr(v->data.variant.payload[i]);
        size_t slen = strlen(s);
        size_t need = pos + slen + 4;
        if (need > cap) {
            while (cap < need) cap *= 2;
            char *nb = (char *)mom_alloc(cap);
            memcpy(nb, buf, pos);
            free(buf);
            buf = nb;
        }
        if (i > 0) { buf[pos++] = ','; buf[pos++] = ' '; }
        memcpy(buf + pos, s, slen);
        pos += slen;
        free(s);
    }
    buf[pos++] = ')';
    buf[pos] = '\0';
    return buf;
}

static char *mom_struct_to_cstr(MomVal *v) {
    /* "Struct{field: val, ...}" */
    size_t cap = 128;
    char *buf = (char *)mom_alloc(cap);
    size_t pos = 0;
    const char *prefix = "Struct{";
    size_t plen = strlen(prefix);
    memcpy(buf, prefix, plen);
    pos = plen;

    for (int i = 0; i < v->data.strct.field_count; i++) {
        const char *fname = v->data.strct.field_names
            ? v->data.strct.field_names[i] : "?";
        char *fval = mom_val_to_cstr(v->data.strct.fields[i]);
        size_t fnlen = strlen(fname);
        size_t fvlen = strlen(fval);
        size_t need = pos + fnlen + fvlen + 6;
        if (need > cap) {
            while (cap < need) cap *= 2;
            char *nb = (char *)mom_alloc(cap);
            memcpy(nb, buf, pos);
            free(buf);
            buf = nb;
        }
        if (i > 0) { buf[pos++] = ','; buf[pos++] = ' '; }
        memcpy(buf + pos, fname, fnlen); pos += fnlen;
        buf[pos++] = ':'; buf[pos++] = ' ';
        memcpy(buf + pos, fval, fvlen); pos += fvlen;
        free(fval);
    }
    if (pos + 2 > cap) {
        char *nb = (char *)mom_alloc(cap + 2);
        memcpy(nb, buf, pos);
        free(buf);
        buf = nb;
    }
    buf[pos++] = '}';
    buf[pos] = '\0';
    return buf;
}

static char *mom_val_to_cstr(MomVal *v) {
    char tmp[64];
    switch (v->tag) {
    case MOM_TAG_INT:
        snprintf(tmp, sizeof(tmp), "%" PRId64, v->data.i);
        return mom_strdup(tmp);
    case MOM_TAG_FLOAT:
        if (v->data.f == (double)(int64_t)v->data.f
            && v->data.f <= 1e15 && v->data.f >= -1e15) {
            snprintf(tmp, sizeof(tmp), "%" PRId64, (int64_t)v->data.f);
        } else {
            snprintf(tmp, sizeof(tmp), "%g", v->data.f);
        }
        return mom_strdup(tmp);
    case MOM_TAG_BOOL:
        return mom_strdup(v->data.b ? "true" : "false");
    case MOM_TAG_STRING:
        return mom_strdup(v->data.s);
    case MOM_TAG_LIST:
        return mom_list_to_cstr(v);
    case MOM_TAG_VARIANT:
        return mom_variant_to_cstr(v);
    case MOM_TAG_STRUCT:
        return mom_struct_to_cstr(v);
    case MOM_TAG_UNIT:
        return mom_strdup("none");
    default:
        return mom_strdup("<unknown>");
    }
}

// ── String operations ─────────────────────────────────────────────────────────

MomVal *mom_str_concat(MomVal *a, MomVal *b) {
    if (a->tag != MOM_TAG_STRING) mom_panic("mom_str_concat: a not a string");
    if (b->tag != MOM_TAG_STRING) mom_panic("mom_str_concat: b not a string");
    return mom_str_owned(mom_strcat_alloc(a->data.s, b->data.s));
}

MomVal *mom_str_concat_c(MomVal *a, const char *b) {
    if (a->tag != MOM_TAG_STRING) mom_panic("mom_str_concat_c: not a string");
    return mom_str_owned(mom_strcat_alloc(a->data.s, b));
}

int mom_str_len(MomVal *v) {
    if (v->tag != MOM_TAG_STRING) mom_panic("mom_str_len: not a string");
    return (int)strlen(v->data.s);
}

MomVal *mom_str_char_at(MomVal *v, int i) {
    if (v->tag != MOM_TAG_STRING) mom_panic("mom_str_char_at: not a string");
    int len = (int)strlen(v->data.s);
    if (i < 0 || i >= len) mom_panic("mom_str_char_at: index out of bounds");
    char buf[2] = { v->data.s[i], '\0' };
    return mom_str(buf);
}

int mom_str_eq(MomVal *a, MomVal *b) {
    if (a->tag != MOM_TAG_STRING || b->tag != MOM_TAG_STRING)
        mom_panic("mom_str_eq: not strings");
    return strcmp(a->data.s, b->data.s) == 0;
}

int mom_str_contains(MomVal *haystack, MomVal *needle) {
    if (haystack->tag != MOM_TAG_STRING) mom_panic("mom_str_contains: not a string");
    if (needle->tag != MOM_TAG_STRING) mom_panic("mom_str_contains: needle not a string");
    return strstr(haystack->data.s, needle->data.s) != NULL;
}

int mom_str_starts_with(MomVal *s, MomVal *prefix) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_starts_with: not a string");
    if (prefix->tag != MOM_TAG_STRING) mom_panic("mom_str_starts_with: prefix not a string");
    size_t plen = strlen(prefix->data.s);
    return strncmp(s->data.s, prefix->data.s, plen) == 0;
}

int mom_str_ends_with(MomVal *s, MomVal *suffix) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_ends_with: not a string");
    if (suffix->tag != MOM_TAG_STRING) mom_panic("mom_str_ends_with: suffix not a string");
    size_t slen = strlen(s->data.s);
    size_t suflen = strlen(suffix->data.s);
    if (suflen > slen) return 0;
    return strcmp(s->data.s + slen - suflen, suffix->data.s) == 0;
}

MomVal *mom_str_slice(MomVal *s, int start, int end) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_slice: not a string");
    int len = (int)strlen(s->data.s);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start > end) start = end;
    int slice_len = end - start;
    char *buf = (char *)mom_alloc((size_t)slice_len + 1);
    memcpy(buf, s->data.s + start, (size_t)slice_len);
    buf[slice_len] = '\0';
    return mom_str_owned(buf);
}

MomVal *mom_str_replace(MomVal *s, MomVal *from, MomVal *to) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_replace: not a string");
    if (from->tag != MOM_TAG_STRING) mom_panic("mom_str_replace: from not a string");
    if (to->tag != MOM_TAG_STRING) mom_panic("mom_str_replace: to not a string");
    const char *src = s->data.s;
    const char *pat = from->data.s;
    const char *rep = to->data.s;
    size_t pat_len = strlen(pat);
    size_t rep_len = strlen(rep);

    if (pat_len == 0) return mom_str(src);

    /* Count occurrences */
    size_t count = 0;
    const char *p = src;
    while ((p = strstr(p, pat)) != NULL) { count++; p += pat_len; }

    size_t src_len = strlen(src);
    size_t out_len = src_len + count * (rep_len - pat_len + (rep_len >= pat_len ? 0 : 0));
    /* safe calculation */
    size_t new_len = src_len - count * pat_len + count * rep_len;
    char *buf = (char *)mom_alloc(new_len + 1);
    char *dst = buf;
    p = src;
    const char *found;
    while ((found = strstr(p, pat)) != NULL) {
        size_t prefix_len = (size_t)(found - p);
        memcpy(dst, p, prefix_len); dst += prefix_len;
        memcpy(dst, rep, rep_len);  dst += rep_len;
        p = found + pat_len;
    }
    size_t tail = strlen(p);
    memcpy(dst, p, tail);
    dst[tail] = '\0';
    (void)out_len;
    return mom_str_owned(buf);
}

MomVal *mom_str_split(MomVal *s, MomVal *delim) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_split: not a string");
    if (delim->tag != MOM_TAG_STRING) mom_panic("mom_str_split: delim not a string");
    MomVal *list = mom_list_new();
    const char *src = s->data.s;
    const char *pat = delim->data.s;
    size_t pat_len = strlen(pat);
    if (pat_len == 0) {
        /* Split every character */
        for (size_t i = 0; src[i]; i++) {
            char buf[2] = { src[i], '\0' };
            mom_list_push(list, mom_str(buf));
        }
        return list;
    }
    const char *p = src;
    const char *found;
    while ((found = strstr(p, pat)) != NULL) {
        size_t len = (size_t)(found - p);
        char *chunk = (char *)mom_alloc(len + 1);
        memcpy(chunk, p, len);
        chunk[len] = '\0';
        mom_list_push(list, mom_str_owned(chunk));
        p = found + pat_len;
    }
    mom_list_push(list, mom_str(p));
    return list;
}

MomVal *mom_str_strip(MomVal *s) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_strip: not a string");
    const char *p = s->data.s;
    while (*p && isspace((unsigned char)*p)) p++;
    const char *end = p + strlen(p);
    while (end > p && isspace((unsigned char)*(end - 1))) end--;
    size_t len = (size_t)(end - p);
    char *buf = (char *)mom_alloc(len + 1);
    memcpy(buf, p, len);
    buf[len] = '\0';
    return mom_str_owned(buf);
}

MomVal *mom_str_upper(MomVal *s) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_upper: not a string");
    char *buf = mom_strdup(s->data.s);
    for (char *p = buf; *p; p++) *p = (char)toupper((unsigned char)*p);
    return mom_str_owned(buf);
}

MomVal *mom_str_lower(MomVal *s) {
    if (s->tag != MOM_TAG_STRING) mom_panic("mom_str_lower: not a string");
    char *buf = mom_strdup(s->data.s);
    for (char *p = buf; *p; p++) *p = (char)tolower((unsigned char)*p);
    return mom_str_owned(buf);
}

MomVal *mom_int_to_str(MomVal *v) {
    if (v->tag != MOM_TAG_INT) mom_panic("mom_int_to_str: not an int");
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId64, v->data.i);
    return mom_str(buf);
}

MomVal *mom_float_to_str(MomVal *v) {
    if (v->tag != MOM_TAG_FLOAT) mom_panic("mom_float_to_str: not a float");
    char buf[64];
    if (v->data.f == (double)(int64_t)v->data.f
        && v->data.f <= 1e15 && v->data.f >= -1e15) {
        snprintf(buf, sizeof(buf), "%" PRId64, (int64_t)v->data.f);
    } else {
        snprintf(buf, sizeof(buf), "%g", v->data.f);
    }
    return mom_str(buf);
}

MomVal *mom_bool_to_str(MomVal *v) {
    if (v->tag != MOM_TAG_BOOL) mom_panic("mom_bool_to_str: not a bool");
    return mom_str(v->data.b ? "true" : "false");
}

// ── I/O ───────────────────────────────────────────────────────────────────────

static void mom_fprint(FILE *fp, MomVal *v) {
    char *s = mom_val_to_cstr(v);
    fputs(s, fp);
    free(s);
}

void mom_print(MomVal *v) {
    mom_fprint(stdout, v);
}

void mom_println(MomVal *v) {
    mom_fprint(stdout, v);
    fputc('\n', stdout);
}

void mom_eprint(MomVal *v) {
    mom_fprint(stderr, v);
}

MomVal *mom_input(MomVal *prompt) {
    if (prompt && prompt->tag == MOM_TAG_STRING && prompt->data.s[0]) {
        fputs(prompt->data.s, stdout);
        fflush(stdout);
    }
    size_t cap = 256;
    char *buf = (char *)mom_alloc(cap);
    size_t pos = 0;
    int c;
    while ((c = fgetc(stdin)) != EOF && c != '\n') {
        if (pos + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)mom_alloc(cap);
            memcpy(nb, buf, pos);
            free(buf);
            buf = nb;
        }
        buf[pos++] = (char)c;
    }
    buf[pos] = '\0';
    return mom_str_owned(buf);
}

MomVal *mom_read_file(MomVal *path) {
    if (path->tag != MOM_TAG_STRING) mom_panic("mom_read_file: path not a string");
    FILE *fp = fopen(path->data.s, "rb");
    if (!fp) {
        fprintf(stderr, "mom: cannot open file '%s': %s\n",
                path->data.s, strerror(errno));
        exit(1);
    }
    fseek(fp, 0, SEEK_END);
    long size = ftell(fp);
    fseek(fp, 0, SEEK_SET);
    char *buf = (char *)mom_alloc((size_t)size + 1);
    size_t read = fread(buf, 1, (size_t)size, fp);
    buf[read] = '\0';
    fclose(fp);
    return mom_str_owned(buf);
}

void mom_write_file(MomVal *path, MomVal *content) {
    if (path->tag != MOM_TAG_STRING) mom_panic("mom_write_file: path not a string");
    if (content->tag != MOM_TAG_STRING) mom_panic("mom_write_file: content not a string");
    FILE *fp = fopen(path->data.s, "wb");
    if (!fp) {
        fprintf(stderr, "mom: cannot write file '%s': %s\n",
                path->data.s, strerror(errno));
        exit(1);
    }
    fputs(content->data.s, fp);
    fclose(fp);
}

MomVal *mom_getenv(MomVal *name) {
    if (name->tag != MOM_TAG_STRING) mom_panic("mom_getenv: not a string");
    const char *val = getenv(name->data.s);
    if (!val) return mom_unit();
    return mom_str(val);
}

// ── Conversion ────────────────────────────────────────────────────────────────

MomVal *mom_to_str(MomVal *v) {
    return mom_str_owned(mom_val_to_cstr(v));
}

MomVal *mom_to_int(MomVal *v) {
    switch (v->tag) {
    case MOM_TAG_INT:    return v;
    case MOM_TAG_FLOAT:  return mom_int((int64_t)v->data.f);
    case MOM_TAG_BOOL:   return mom_int((int64_t)v->data.b);
    case MOM_TAG_STRING: {
        char *end;
        int64_t n = (int64_t)strtoll(v->data.s, &end, 10);
        if (end == v->data.s) mom_panic("mom_to_int: cannot parse string as int");
        return mom_int(n);
    }
    default: mom_panic("mom_to_int: unsupported type"); return NULL;
    }
}

MomVal *mom_to_float(MomVal *v) {
    switch (v->tag) {
    case MOM_TAG_FLOAT:  return v;
    case MOM_TAG_INT:    return mom_float((double)v->data.i);
    case MOM_TAG_BOOL:   return mom_float((double)v->data.b);
    case MOM_TAG_STRING: {
        char *end;
        double f = strtod(v->data.s, &end);
        if (end == v->data.s) mom_panic("mom_to_float: cannot parse string as float");
        return mom_float(f);
    }
    default: mom_panic("mom_to_float: unsupported type"); return NULL;
    }
}

MomVal *mom_to_bool(MomVal *v) {
    switch (v->tag) {
    case MOM_TAG_BOOL:   return v;
    case MOM_TAG_INT:    return mom_bool(v->data.i != 0);
    case MOM_TAG_FLOAT:  return mom_bool(v->data.f != 0.0);
    case MOM_TAG_STRING: return mom_bool(v->data.s[0] != '\0');
    case MOM_TAG_LIST:   return mom_bool(v->data.list.len != 0);
    case MOM_TAG_UNIT:   return mom_bool(0);
    default: return mom_bool(1);
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

MomVal *mom_add(MomVal *a, MomVal *b) {
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT)
        return mom_int(a->data.i + b->data.i);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT)
        return mom_float(a->data.f + b->data.f);
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT)
        return mom_float((double)a->data.i + b->data.f);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT)
        return mom_float(a->data.f + (double)b->data.i);
    if (a->tag == MOM_TAG_STRING && b->tag == MOM_TAG_STRING)
        return mom_str_owned(mom_strcat_alloc(a->data.s, b->data.s));
    mom_panic("mom_add: unsupported operand types");
    return NULL;
}

MomVal *mom_sub(MomVal *a, MomVal *b) {
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT)
        return mom_int(a->data.i - b->data.i);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT)
        return mom_float(a->data.f - b->data.f);
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT)
        return mom_float((double)a->data.i - b->data.f);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT)
        return mom_float(a->data.f - (double)b->data.i);
    mom_panic("mom_sub: unsupported operand types");
    return NULL;
}

MomVal *mom_mul(MomVal *a, MomVal *b) {
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT)
        return mom_int(a->data.i * b->data.i);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT)
        return mom_float(a->data.f * b->data.f);
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT)
        return mom_float((double)a->data.i * b->data.f);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT)
        return mom_float(a->data.f * (double)b->data.i);
    mom_panic("mom_mul: unsupported operand types");
    return NULL;
}

MomVal *mom_div(MomVal *a, MomVal *b) {
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT) {
        if (b->data.i == 0) mom_panic("mom_div: division by zero");
        return mom_int(a->data.i / b->data.i);
    }
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT)
        return mom_float(a->data.f / b->data.f);
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT)
        return mom_float((double)a->data.i / b->data.f);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT)
        return mom_float(a->data.f / (double)b->data.i);
    mom_panic("mom_div: unsupported operand types");
    return NULL;
}

MomVal *mom_mod(MomVal *a, MomVal *b) {
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT) {
        if (b->data.i == 0) mom_panic("mom_mod: modulo by zero");
        return mom_int(a->data.i % b->data.i);
    }
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT) {
        double result = a->data.f - (double)(int64_t)(a->data.f / b->data.f) * b->data.f;
        return mom_float(result);
    }
    mom_panic("mom_mod: unsupported operand types");
    return NULL;
}

MomVal *mom_neg(MomVal *a) {
    if (a->tag == MOM_TAG_INT)   return mom_int(-a->data.i);
    if (a->tag == MOM_TAG_FLOAT) return mom_float(-a->data.f);
    mom_panic("mom_neg: not a number");
    return NULL;
}

MomVal *mom_not(MomVal *a) {
    if (a->tag == MOM_TAG_BOOL) return mom_bool(!a->data.b);
    mom_panic("mom_not: not a bool");
    return NULL;
}

// Deep equality

static int mom_val_eq(MomVal *a, MomVal *b) {
    if (a->tag != b->tag) {
        /* int/float cross-comparison */
        if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT)
            return (double)a->data.i == b->data.f;
        if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT)
            return a->data.f == (double)b->data.i;
        return 0;
    }
    switch (a->tag) {
    case MOM_TAG_INT:    return a->data.i == b->data.i;
    case MOM_TAG_FLOAT:  return a->data.f == b->data.f;
    case MOM_TAG_BOOL:   return a->data.b == b->data.b;
    case MOM_TAG_STRING: return strcmp(a->data.s, b->data.s) == 0;
    case MOM_TAG_UNIT:   return 1;
    case MOM_TAG_LIST:
        if (a->data.list.len != b->data.list.len) return 0;
        for (int i = 0; i < a->data.list.len; i++) {
            if (!mom_val_eq(a->data.list.items[i], b->data.list.items[i])) return 0;
        }
        return 1;
    case MOM_TAG_VARIANT:
        if (a->data.variant.variant_id != b->data.variant.variant_id) return 0;
        if (a->data.variant.payload_len != b->data.variant.payload_len) return 0;
        for (int i = 0; i < a->data.variant.payload_len; i++) {
            if (!mom_val_eq(a->data.variant.payload[i], b->data.variant.payload[i])) return 0;
        }
        return 1;
    case MOM_TAG_STRUCT:
        if (a->data.strct.type_id != b->data.strct.type_id) return 0;
        if (a->data.strct.field_count != b->data.strct.field_count) return 0;
        for (int i = 0; i < a->data.strct.field_count; i++) {
            if (!mom_val_eq(a->data.strct.fields[i], b->data.strct.fields[i])) return 0;
        }
        return 1;
    default: return 0;
    }
}

MomVal *mom_eq(MomVal *a, MomVal *b) { return mom_bool(mom_val_eq(a, b)); }
MomVal *mom_ne(MomVal *a, MomVal *b) { return mom_bool(!mom_val_eq(a, b)); }

static int mom_val_cmp(MomVal *a, MomVal *b) {
    /* returns <0, 0, >0 */
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_INT)
        return (a->data.i > b->data.i) - (a->data.i < b->data.i);
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_FLOAT)
        return (a->data.f > b->data.f) - (a->data.f < b->data.f);
    if (a->tag == MOM_TAG_INT && b->tag == MOM_TAG_FLOAT) {
        double af = (double)a->data.i;
        return (af > b->data.f) - (af < b->data.f);
    }
    if (a->tag == MOM_TAG_FLOAT && b->tag == MOM_TAG_INT) {
        double bf = (double)b->data.i;
        return (a->data.f > bf) - (a->data.f < bf);
    }
    if (a->tag == MOM_TAG_STRING && b->tag == MOM_TAG_STRING)
        return strcmp(a->data.s, b->data.s);
    mom_panic("mom comparison: unsupported types");
    return 0;
}

MomVal *mom_lt(MomVal *a, MomVal *b) { return mom_bool(mom_val_cmp(a, b) <  0); }
MomVal *mom_le(MomVal *a, MomVal *b) { return mom_bool(mom_val_cmp(a, b) <= 0); }
MomVal *mom_gt(MomVal *a, MomVal *b) { return mom_bool(mom_val_cmp(a, b) >  0); }
MomVal *mom_ge(MomVal *a, MomVal *b) { return mom_bool(mom_val_cmp(a, b) >= 0); }

// ── Range ─────────────────────────────────────────────────────────────────────

MomVal *mom_range(int64_t n) {
    return mom_range2(0, n);
}

MomVal *mom_range2(int64_t start, int64_t end) {
    return mom_range3(start, end, 1);
}

MomVal *mom_range3(int64_t start, int64_t end, int64_t step) {
    if (step == 0) mom_panic("mom_range3: step cannot be zero");
    MomVal *list = mom_list_new();
    if (step > 0) {
        for (int64_t i = start; i < end; i += step)
            mom_list_push(list, mom_int(i));
    } else {
        for (int64_t i = start; i > end; i += step)
            mom_list_push(list, mom_int(i));
    }
    return list;
}

// ── Named struct field access ─────────────────────────────────────────────────

MomVal *mom_struct_get_named(MomVal *v, const char *name) {
    if (v->tag != MOM_TAG_STRUCT) mom_panic("mom_struct_get_named: not a struct");
    for (int i = 0; i < v->data.strct.field_count; i++) {
        if (v->data.strct.field_names && strcmp(v->data.strct.field_names[i], name) == 0)
            return v->data.strct.fields[i];
    }
    char errbuf[256];
    snprintf(errbuf, sizeof(errbuf), "mom_struct_get_named: field '%s' not found", name);
    mom_panic(errbuf);
    return NULL;
}

void mom_struct_set_named(MomVal *v, const char *name, MomVal *val) {
    if (v->tag != MOM_TAG_STRUCT) mom_panic("mom_struct_set_named: not a struct");
    for (int i = 0; i < v->data.strct.field_count; i++) {
        if (v->data.strct.field_names && strcmp(v->data.strct.field_names[i], name) == 0) {
            v->data.strct.fields[i] = val;
            return;
        }
    }
    char errbuf[256];
    snprintf(errbuf, sizeof(errbuf), "mom_struct_set_named: field '%s' not found", name);
    mom_panic(errbuf);
}

// ── Stage-1.3 runtime helpers ─────────────────────────────────────────────────

int64_t mom_val_len(MomVal *v) {
    if (v->tag == MOM_TAG_STRING) return (int64_t)strlen(v->data.s);
    if (v->tag == MOM_TAG_LIST)   return (int64_t)v->data.list.len;
    mom_panic("mom_val_len: expected String or List");
    return 0;
}

MomVal *mom_val_index(MomVal *v, MomVal *idx) {
    int64_t i = idx->tag == MOM_TAG_INT ? idx->data.i : (int64_t)idx->data.f;
    if (v->tag == MOM_TAG_STRING) return mom_str_char_at(v, (int)i);
    if (v->tag == MOM_TAG_LIST)   return mom_list_get(v, (int)i);
    mom_panic("mom_val_index: not indexable");
    return NULL;
}

MomVal *mom_and(MomVal *a, MomVal *b) {
    return mom_bool(mom_bool_val(a) && mom_bool_val(b));
}

MomVal *mom_or(MomVal *a, MomVal *b) {
    return mom_bool(mom_bool_val(a) || mom_bool_val(b));
}

MomVal *mom_pop_opt(MomVal *list) {
    if (list->tag != MOM_TAG_LIST) mom_panic("mom_pop_opt: not a list");
    if (list->data.list.len == 0)
        return mom_variant(MOM_OPT_None, 0);
    return mom_variant(MOM_OPT_Some, 1, mom_list_pop(list));
}

MomVal *mom_getenv_opt(MomVal *name) {
    if (name->tag != MOM_TAG_STRING) mom_panic("mom_getenv_opt: not a string");
    const char *val = getenv(name->data.s);
    if (!val) return mom_variant(MOM_OPT_None, 0);
    return mom_variant(MOM_OPT_Some, 1, mom_str(val));
}

MomVal *mom_read_file_result(MomVal *path) {
    if (path->tag != MOM_TAG_STRING) mom_panic("mom_read_file_result: not a string");
    FILE *fp = fopen(path->data.s, "rb");
    if (!fp) {
        char errbuf[512];
        snprintf(errbuf, sizeof(errbuf), "cannot open '%s': %s",
                 path->data.s, strerror(errno));
        return mom_variant(MOM_RES_Err, 1, mom_str(errbuf));
    }
    fseek(fp, 0, SEEK_END);
    long size = ftell(fp);
    fseek(fp, 0, SEEK_SET);
    char *buf = (char *)mom_alloc((size_t)size + 1);
    size_t rd = fread(buf, 1, (size_t)size, fp);
    buf[rd] = '\0';
    fclose(fp);
    return mom_variant(MOM_RES_Ok, 1, mom_str_owned(buf));
}

MomVal *mom_write_file_result(MomVal *path, MomVal *content) {
    if (path->tag != MOM_TAG_STRING)    mom_panic("mom_write_file_result: path not a string");
    if (content->tag != MOM_TAG_STRING) mom_panic("mom_write_file_result: content not a string");
    FILE *fp = fopen(path->data.s, "wb");
    if (!fp) {
        char errbuf[512];
        snprintf(errbuf, sizeof(errbuf), "cannot write '%s': %s",
                 path->data.s, strerror(errno));
        return mom_variant(MOM_RES_Err, 1, mom_str(errbuf));
    }
    fputs(content->data.s, fp);
    fclose(fp);
    return mom_variant(MOM_RES_Ok, 1, mom_unit());
}

void mom_eprint_val(MomVal *v) {
    char *s = mom_val_to_cstr(v);
    fputs(s, stderr);
    free(s);
}

MomVal *mom_to_string(MomVal *v) { return mom_to_str(v); }

int64_t mom_int_from_val(MomVal *v) {
    if (v->tag == MOM_TAG_INT)  return v->data.i;
    if (v->tag == MOM_TAG_BOOL) return (int64_t)v->data.b;
    mom_panic("mom_int_from_val: not an int");
    return 0;
}

int mom_bool_from_val(MomVal *v) {
    if (v->tag == MOM_TAG_BOOL) return v->data.b;
    if (v->tag == MOM_TAG_INT)  return v->data.i != 0;
    mom_panic("mom_bool_from_val: not a bool");
    return 0;
}

// ── Stage-1 native print helpers ──────────────────────────────────────────────
// Used by programs compiled by the stage-1 mom-in-mom compiler.
// These work with raw C types, not MomVal*, for efficiency.

void mom_print_int(int64_t n) {
    printf("%" PRId64 "\n", n);
}

void mom_print_bool(int b) {
    printf("%s\n", b ? "true" : "false");
}

void mom_print_unit(void) {
    printf("()\n");
}

void mom_print_str(const char *s) {
    /* stage-1 `print(s: String)` prints the string and a newline,
     * matching the behaviour of mom_print_int/bool. */
    if (s) fputs(s, stdout);
    fputc('\n', stdout);
}

const char *mom_str_from_int(int64_t n) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%" PRId64, n);
    return mom_strdup(buf);
}

const char *mom_str_from_bool(int b) {
    return mom_strdup(b ? "true" : "false");
}

int64_t mom_str_len_raw(const char *s) {
    return s ? (int64_t)strlen(s) : 0;
}

int64_t mom_digit_value(const char *c) {
    if (!c || !*c) return -1;
    unsigned char ch = (unsigned char)c[0];
    if (ch >= '0' && ch <= '9') return (int64_t)(ch - '0');
    return -1;
}

// ── Entry point ───────────────────────────────────────────────────────────────
// The stage-1 compiler emits mom_main() instead of main().
// This wrapper calls it so the binary can be executed directly.

int main(void) {
    mom_main();
    return 0;
}
