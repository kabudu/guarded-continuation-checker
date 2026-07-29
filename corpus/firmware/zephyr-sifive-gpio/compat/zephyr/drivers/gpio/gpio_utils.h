#ifndef GCC_ZEPHYR_GPIO_UTILS_H
#define GCC_ZEPHYR_GPIO_UTILS_H

static inline int gpio_manage_callback(sys_slist_t *callbacks,
                                       struct gpio_callback *callback,
                                       bool set) {
  (void)callbacks;
  (void)callback;
  (void)set;
  return 0;
}

static inline void gpio_fire_callbacks(sys_slist_t *callbacks,
                                       const struct device *device,
                                       gpio_port_pins_t pins) {
  (void)callbacks;
  (void)device;
  (void)pins;
}

#endif
