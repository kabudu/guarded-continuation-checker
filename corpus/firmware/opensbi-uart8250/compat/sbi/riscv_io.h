#ifndef GCC_OPENSBI_RISCV_IO_H
#define GCC_OPENSBI_RISCV_IO_H

#include <stdint.h>

static inline uint8_t readb(const volatile void *address) {
  return *(const volatile uint8_t *)address;
}

static inline uint16_t readw(const volatile void *address) {
  return *(const volatile uint16_t *)address;
}

static inline uint32_t readl(const volatile void *address) {
  return *(const volatile uint32_t *)address;
}

static inline void writeb(uint32_t value, volatile void *address) {
  *(volatile uint8_t *)address = (uint8_t)value;
}

static inline void writew(uint32_t value, volatile void *address) {
  *(volatile uint16_t *)address = (uint16_t)value;
}

static inline void writel(uint32_t value, volatile void *address) {
  *(volatile uint32_t *)address = value;
}

#endif
