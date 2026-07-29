#ifndef GCC_ZEPHYR_KERNEL_H
#define GCC_ZEPHYR_KERNEL_H

#include <stdbool.h>
#include <stdint.h>

#define __ASSERT_NO_MSG(condition) ((void)(condition))
#define __ASSERT(condition, ...) ((void)(condition))

typedef struct {
  uintptr_t opaque;
} sys_slist_t;

#endif
