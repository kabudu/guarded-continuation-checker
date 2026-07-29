#ifndef GCC_ZEPHYR_IRQ_H
#define GCC_ZEPHYR_IRQ_H

static inline unsigned int irq_get_level(unsigned int irq) {
  (void)irq;
  return 1;
}

static inline void irq_enable(unsigned int irq) { (void)irq; }
static inline void irq_disable(unsigned int irq) { (void)irq; }

#define IRQ_CONNECT(...)

#endif
