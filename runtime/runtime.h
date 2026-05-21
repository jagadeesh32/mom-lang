/* mom runtime — Phase 1 minimal C runtime.
 *
 * The native code generator emits programs that include this header
 * and link with `runtime.c`. Functions here are the surface the
 * generated code is allowed to call.
 */

#ifndef MOM_RUNTIME_H
#define MOM_RUNTIME_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Output primitives — must match the helpers chosen by codegen.rs. */
void mom_print_int(int64_t value);
void mom_print_bool(bool value);
void mom_print_float(double value);
void mom_print_unit(void);

/* Entry point implemented by user code (renamed from `main`). */
void mom_main(void);

#ifdef __cplusplus
}
#endif

#endif /* MOM_RUNTIME_H */
