#ifndef GCC_OPENTITAN_COMPAT_DIF_BASE_H_
#define GCC_OPENTITAN_COMPAT_DIF_BASE_H_

#include <stdbool.h>

typedef enum dif_result {
  kDifOk = 0,
  kDifError = 1,
  kDifBadArg = 2,
  kDifLocked = 3,
} dif_result_t;

typedef enum dif_toggle {
  kDifToggleDisabled = 0,
  kDifToggleEnabled = 1,
} dif_toggle_t;

static inline bool dif_is_valid_toggle(dif_toggle_t toggle) {
  return toggle == kDifToggleDisabled || toggle == kDifToggleEnabled;
}

static inline bool dif_toggle_to_bool(dif_toggle_t toggle) {
  return toggle == kDifToggleEnabled;
}

static inline dif_toggle_t dif_bool_to_toggle(bool value) {
  return value ? kDifToggleEnabled : kDifToggleDisabled;
}

#endif
