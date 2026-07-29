#ifndef GCC_ZEPHYR_DEVICE_H
#define GCC_ZEPHYR_DEVICE_H

struct device {
  const void *config;
  void *data;
};

#define DEVICE_API(kind, name) const struct gpio_driver_api name
#define DEVICE_DT_INST_DEFINE(...)
#define DEVICE_DT_INST_GET(index) ((const struct device *)0)
#define PRE_KERNEL_1 0
#define CONFIG_GPIO_INIT_PRIORITY 0
#define DT_INST_REG_ADDR(index) 0
#define DT_INST_IRQN(index) 0
#define DT_INST_IRQN_BY_IDX(index, irq) 0
#define DT_INST_IRQ_BY_IDX(index, irq, cell) 0
#define DT_INST_IRQ_HAS_IDX(index, irq) 0

#endif
