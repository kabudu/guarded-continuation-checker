#include <stddef.h>
#include <stdint.h>

#include "hw/top/pwm_regs.h"
#include "sw/device/lib/dif/dif_pwm.h"

typedef struct gcc_mmio_event {
  uint32_t operation;
  uint32_t offset;
  uint32_t value;
} gcc_mmio_event_t;

enum {
  kGccMmioRead = 1,
  kGccMmioWrite = 2,
  kGccObserveChannel0 = 3,
  kGccMaximumEvents = 32,
};

volatile gcc_mmio_event_t gcc_mmio_events[kGccMaximumEvents];
volatile uint32_t gcc_mmio_event_count;
volatile uint32_t gcc_pwm_enable_register;
volatile uint32_t gcc_pwm_invert_register;

static void record_event(uint32_t operation, uint32_t offset, uint32_t value) {
  uint32_t index = gcc_mmio_event_count;
  if (index < kGccMaximumEvents) {
    gcc_mmio_events[index].operation = operation;
    gcc_mmio_events[index].offset = offset;
    gcc_mmio_events[index].value = value;
    gcc_mmio_event_count = index + 1;
  }
}

__attribute__((noinline, used)) uint32_t mmio_region_read32(
    mmio_region_t region, ptrdiff_t offset) {
  (void)region;
  uint32_t value = 0;
  switch (offset) {
    case PWM_REGWEN_REG_OFFSET:
      value = 1;
      break;
    case PWM_CFG_REG_OFFSET:
      value = 3;
      break;
    case PWM_PWM_EN_REG_OFFSET:
      value = gcc_pwm_enable_register;
      break;
    case PWM_INVERT_REG_OFFSET:
      value = gcc_pwm_invert_register;
      break;
    default:
      value = 0;
      break;
  }
  record_event(kGccMmioRead, (uint32_t)offset, value);
  return value;
}

__attribute__((noinline, used)) void mmio_region_write32(
    mmio_region_t region, ptrdiff_t offset, uint32_t value) {
  (void)region;
  if (offset == PWM_PWM_EN_REG_OFFSET) {
    gcc_pwm_enable_register = value;
  } else if (offset == PWM_INVERT_REG_OFFSET) {
    gcc_pwm_invert_register = value;
  }
  record_event(kGccMmioWrite, (uint32_t)offset, value);
}

__attribute__((noinline, used)) void gcc_observe_channel0(void) {
  record_event(kGccObserveChannel0, 0, gcc_pwm_enable_register & 1);
}

#ifdef GCC_RUNTIME_CHANNEL_CONTROL
__attribute__((used)) uint32_t gcc_firmware_entry(uint32_t runtime_channel) {
#else
__attribute__((used)) uint32_t gcc_firmware_entry(void) {
#endif
  const dif_pwm_t pwm = {.base_addr = {.base = UINT32_C(0x40000000)}};
  const dif_pwm_channel_config_t channel0 = {
      .polarity = kDifPwmPolarityActiveHigh,
      .mode = kDifPwmModeFirmware,
      .duty_cycle_a = 4,
      .duty_cycle_b = 8,
      .phase_delay = 0,
      .blink_parameter_x = 0,
      .blink_parameter_y = 0,
  };
  const dif_pwm_channel_config_t channel1 = {
      .polarity = kDifPwmPolarityActiveHigh,
      .mode = kDifPwmModeFirmware,
      .duty_cycle_a = 6,
      .duty_cycle_b = 10,
      .phase_delay = 2,
      .blink_parameter_x = 0,
      .blink_parameter_y = 0,
  };
  uint32_t result = 0;
  result |= (uint32_t)dif_pwm_configure_channel(&pwm, 0, channel0);
  result |= (uint32_t)dif_pwm_channels_set_enabled(
      &pwm, UINT32_C(1) << 0, kDifToggleEnabled);
#ifdef GCC_RUNTIME_CHANNEL_CONTROL
  result |=
      (uint32_t)dif_pwm_configure_channel(&pwm, runtime_channel, channel1);
#else
  result |= (uint32_t)dif_pwm_configure_channel(&pwm, 1, channel1);
#endif
  gcc_observe_channel0();
  return result;
}
