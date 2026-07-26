#ifndef GCC_OPENTITAN_COMPAT_DIF_PWM_H_
#define GCC_OPENTITAN_COMPAT_DIF_PWM_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sw/device/lib/dif/dif_base.h"

typedef struct mmio_region {
  uintptr_t base;
} mmio_region_t;

uint32_t mmio_region_read32(mmio_region_t region, ptrdiff_t offset);
void mmio_region_write32(mmio_region_t region, ptrdiff_t offset,
                         uint32_t value);

typedef struct dif_pwm {
  mmio_region_t base_addr;
} dif_pwm_t;

typedef uint32_t dif_pwm_channel_t;

typedef enum dif_pwm_polarity {
  kDifPwmPolarityActiveHigh = 0,
  kDifPwmPolarityActiveLow = 1,
} dif_pwm_polarity_t;

typedef enum dif_pwm_mode {
  kDifPwmModeFirmware = 0,
  kDifPwmModeBlink = 1,
  kDifPwmModeHeartbeat = 2,
} dif_pwm_mode_t;

typedef struct dif_pwm_config {
  uint32_t clock_divisor;
  uint32_t beats_per_pulse_cycle;
} dif_pwm_config_t;

typedef struct dif_pwm_channel_config {
  dif_pwm_polarity_t polarity;
  dif_pwm_mode_t mode;
  uint32_t duty_cycle_a;
  uint32_t duty_cycle_b;
  uint32_t phase_delay;
  uint32_t blink_parameter_x;
  uint32_t blink_parameter_y;
} dif_pwm_channel_config_t;

dif_result_t dif_pwm_configure(const dif_pwm_t *pwm, dif_pwm_config_t config);
dif_result_t dif_pwm_configure_channel(const dif_pwm_t *pwm,
                                       dif_pwm_channel_t channel,
                                       dif_pwm_channel_config_t config);
dif_result_t dif_pwm_phase_cntr_set_enabled(const dif_pwm_t *pwm,
                                            dif_toggle_t enabled);
dif_result_t dif_pwm_phase_cntr_get_enabled(const dif_pwm_t *pwm,
                                            dif_toggle_t *is_enabled);
dif_result_t dif_pwm_channels_set_enabled(const dif_pwm_t *pwm,
                                          uint32_t channels,
                                          dif_toggle_t enabled);
dif_result_t dif_pwm_channel_get_enabled(const dif_pwm_t *pwm,
                                         dif_pwm_channel_t channel,
                                         dif_toggle_t *is_enabled);
dif_result_t dif_pwm_lock(const dif_pwm_t *pwm);
dif_result_t dif_pwm_is_locked(const dif_pwm_t *pwm, bool *is_locked);

#endif
