#ifndef GCC_ZEPHYR_IRQ_MULTILEVEL_H
#define GCC_ZEPHYR_IRQ_MULTILEVEL_H

#define CONFIG_1ST_LEVEL_INTERRUPT_BITS 8

static inline unsigned int irq_from_level_2(unsigned int irq) {
  return irq >> CONFIG_1ST_LEVEL_INTERRUPT_BITS;
}

#endif
