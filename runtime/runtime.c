/* mom runtime — Phase 1 minimal C runtime.
 *
 * Provides:
 *   - `mom_print_int` / `mom_print_bool` / `mom_print_unit`
 *   - `main` shim that invokes user-defined `mom_main`
 */

#include "runtime.h"

#include <inttypes.h>
#include <stdio.h>

void mom_print_int(int64_t value) {
    printf("%" PRId64 "\n", value);
}

void mom_print_bool(bool value) {
    fputs(value ? "true\n" : "false\n", stdout);
}

void mom_print_float(double value) {
    /* Match the interpreter's `Display` impl for Float: Rust's default
     * f64 formatter prints whole numbers without a trailing decimal,
     * so we replicate that here for bit-identical test oracles. */
    if (value == (double)(int64_t)value
        && value <= 1e15 && value >= -1e15) {
        printf("%" PRId64 "\n", (int64_t)value);
    } else {
        printf("%g\n", value);
    }
}

void mom_print_unit(void) {
    fputs("()\n", stdout);
}

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;
    mom_main();
    return 0;
}
