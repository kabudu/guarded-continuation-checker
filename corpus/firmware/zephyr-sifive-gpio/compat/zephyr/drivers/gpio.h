#ifndef GCC_ZEPHYR_GPIO_H
#define GCC_ZEPHYR_GPIO_H

#include <stdbool.h>
#include <stdint.h>

typedef uint32_t gpio_pin_t;
typedef uint32_t gpio_flags_t;
typedef uint32_t gpio_port_pins_t;
typedef uint32_t gpio_port_value_t;

enum gpio_int_mode {
  GPIO_INT_MODE_DISABLED = 0,
  GPIO_INT_MODE_LEVEL = 1,
  GPIO_INT_MODE_EDGE = 2,
};

enum gpio_int_trig {
  GPIO_INT_TRIG_LOW = 1,
  GPIO_INT_TRIG_HIGH = 2,
  GPIO_INT_TRIG_BOTH = 3,
};

enum {
  GPIO_INPUT = 1U << 0,
  GPIO_OUTPUT = 1U << 1,
  GPIO_OUTPUT_INIT_LOW = 1U << 2,
  GPIO_OUTPUT_INIT_HIGH = 1U << 3,
  GPIO_PULL_UP = 1U << 4,
  GPIO_PULL_DOWN = 1U << 5,
  GPIO_SINGLE_ENDED = 1U << 6,
  GPIO_INT_LOW_0 = 1U,
  GPIO_INT_HIGH_1 = 2U,
};

struct gpio_driver_config {
  gpio_port_pins_t port_pin_mask;
};

struct gpio_driver_data {
  uint32_t unused;
};

struct gpio_callback {
  uint32_t unused;
};

struct gpio_driver_api {
  int (*pin_configure)(const struct device *, gpio_pin_t, gpio_flags_t);
  int (*port_get_raw)(const struct device *, gpio_port_value_t *);
  int (*port_set_masked_raw)(const struct device *, gpio_port_pins_t,
                             gpio_port_value_t);
  int (*port_set_bits_raw)(const struct device *, gpio_port_pins_t);
  int (*port_clear_bits_raw)(const struct device *, gpio_port_pins_t);
  int (*port_toggle_bits)(const struct device *, gpio_port_pins_t);
  int (*pin_interrupt_configure)(const struct device *, gpio_pin_t,
                                 enum gpio_int_mode, enum gpio_int_trig);
  int (*manage_callback)(const struct device *, struct gpio_callback *, bool);
};

#define GPIO_PORT_PIN_MASK_FROM_DT_INST(index) UINT32_MAX

#endif
