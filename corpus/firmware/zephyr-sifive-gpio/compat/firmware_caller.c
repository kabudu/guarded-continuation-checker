#include <stdint.h>

#include "upstream/gpio_sifive.c"

typedef struct gcc_mmio_event {
  uint32_t operation;
  uint32_t offset;
  uint32_t value;
} gcc_mmio_event_t;

enum {
  kGccMmioState = 4,
  kGccMaximumEvents = 32,
};

volatile gcc_mmio_event_t gcc_mmio_events[kGccMaximumEvents];
volatile uint32_t gcc_mmio_event_count;
static volatile struct gpio_sifive_t gcc_gpio_registers;
static struct gpio_sifive_data gcc_gpio_data;

static const struct gpio_sifive_config gcc_gpio_config = {
    .common = {.port_pin_mask = UINT32_MAX},
    .gpio_base_addr = (uintptr_t)&gcc_gpio_registers,
    .gpio_irq_base = 0,
    .gpio_cfg_func = gpio_sifive_cfg_0,
};

static struct device gcc_gpio_device = {
    .config = &gcc_gpio_config,
    .data = &gcc_gpio_data,
};

static void record_state(uint32_t offset, uint32_t value) {
  uint32_t index = gcc_mmio_event_count;
  if (index < kGccMaximumEvents) {
    gcc_mmio_events[index].operation = kGccMmioState;
    gcc_mmio_events[index].offset = offset;
    gcc_mmio_events[index].value = value;
    gcc_mmio_event_count = index + 1;
  }
}

__attribute__((used)) uint32_t gcc_firmware_entry(uint32_t input) {
  gpio_pin_t pin = input & 7U;
  gpio_port_pins_t mask = UINT32_C(1) << pin;
  gpio_flags_t flags = GPIO_OUTPUT;
  int result;

  if ((input & UINT32_C(0x08)) != 0) {
    flags |= GPIO_OUTPUT_INIT_HIGH;
  } else {
    flags |= GPIO_OUTPUT_INIT_LOW;
  }
  if ((input & UINT32_C(0x10)) != 0) {
    flags |= GPIO_PULL_UP;
  }

  result = gpio_sifive_config(&gcc_gpio_device, pin, flags);
  switch ((input >> 5) & 3U) {
    case 0:
      result |= gpio_sifive_port_set_bits_raw(&gcc_gpio_device, mask);
      break;
    case 1:
      result |= gpio_sifive_port_clear_bits_raw(&gcc_gpio_device, mask);
      break;
    case 2:
      result |= gpio_sifive_port_toggle_bits(&gcc_gpio_device, mask);
      break;
    default:
      result |= gpio_sifive_port_set_masked_raw(
          &gcc_gpio_device, UINT32_C(0xff), input);
      break;
  }

  record_state(2U, gcc_gpio_registers.out_en);
  record_state(3U, gcc_gpio_registers.out_val);
  record_state(4U, gcc_gpio_registers.pue);
  return (uint32_t)result;
}
