#ifndef GCC_ZEPHYR_UTIL_H
#define GCC_ZEPHYR_UTIL_H

#define BIT(index) (UINT32_C(1) << (index))
#define WRITE_BIT(value, bit, set) \
  ((value) = (set) ? ((value) | BIT(bit)) : ((value) & ~BIT(bit)))

#endif
