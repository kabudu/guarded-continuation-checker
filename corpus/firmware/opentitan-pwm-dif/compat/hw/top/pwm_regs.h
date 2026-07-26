#ifndef GCC_OPENTITAN_COMPAT_PWM_REGS_H_
#define GCC_OPENTITAN_COMPAT_PWM_REGS_H_

#include "sw/device/lib/base/bitfield.h"

#define PWM_PARAM_N_OUTPUTS 6

#define PWM_REGWEN_REG_OFFSET 0x04
#define PWM_CFG_REG_OFFSET 0x08
#define PWM_PWM_EN_REG_OFFSET 0x0c
#define PWM_INVERT_REG_OFFSET 0x10
#define PWM_PWM_PARAM_0_REG_OFFSET 0x14
#define PWM_DUTY_CYCLE_0_REG_OFFSET 0x2c
#define PWM_BLINK_PARAM_0_REG_OFFSET 0x44

#define PWM_CFG_CLK_DIV_MASK 0x07ffffffu
#define PWM_CFG_CLK_DIV_FIELD \
  ((bitfield_field32_t){.mask = PWM_CFG_CLK_DIV_MASK, .index = 4})
#define PWM_CFG_DC_RESN_MASK 0x0fu
#define PWM_CFG_DC_RESN_FIELD \
  ((bitfield_field32_t){.mask = PWM_CFG_DC_RESN_MASK, .index = 0})
#define PWM_CFG_CNTR_EN_BIT 31

#define PWM_PWM_EN_EN_0_BIT 0
#define PWM_INVERT_INVERT_0_BIT 0

#define PWM_DUTY_CYCLE_0_A_0_FIELD \
  ((bitfield_field32_t){.mask = 0xffffu, .index = 0})
#define PWM_DUTY_CYCLE_0_B_0_FIELD \
  ((bitfield_field32_t){.mask = 0xffffu, .index = 16})

#define PWM_PWM_PARAM_0_PHASE_DELAY_0_FIELD \
  ((bitfield_field32_t){.mask = 0xffffu, .index = 0})
#define PWM_PWM_PARAM_0_BLINK_EN_0_BIT 31
#define PWM_PWM_PARAM_0_HTBT_EN_0_BIT 30

#define PWM_BLINK_PARAM_0_X_0_FIELD \
  ((bitfield_field32_t){.mask = 0xffffu, .index = 0})
#define PWM_BLINK_PARAM_0_Y_0_FIELD \
  ((bitfield_field32_t){.mask = 0xffffu, .index = 16})

#endif
