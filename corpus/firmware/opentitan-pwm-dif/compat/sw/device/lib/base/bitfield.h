#ifndef GCC_OPENTITAN_COMPAT_BITFIELD_H_
#define GCC_OPENTITAN_COMPAT_BITFIELD_H_

#include <stdbool.h>
#include <stdint.h>

typedef struct bitfield_field32 {
  uint32_t mask;
  uint32_t index;
} bitfield_field32_t;

static inline uint32_t bitfield_field32_write(uint32_t value,
                                               bitfield_field32_t field,
                                               uint32_t field_value) {
  value &= ~(field.mask << field.index);
  value |= (field_value & field.mask) << field.index;
  return value;
}

static inline uint32_t bitfield_field32_read(uint32_t value,
                                              bitfield_field32_t field) {
  return (value >> field.index) & field.mask;
}

static inline uint32_t bitfield_bit32_write(uint32_t value, uint32_t bit,
                                             bool set) {
  uint32_t mask = UINT32_C(1) << bit;
  return set ? value | mask : value & ~mask;
}

static inline bool bitfield_bit32_read(uint32_t value, uint32_t bit) {
  return ((value >> bit) & UINT32_C(1)) != 0;
}

static inline int32_t bitfield_count_leading_zeroes32(uint32_t value) {
  return value == 0 ? 32 : __builtin_clz(value);
}

#endif
