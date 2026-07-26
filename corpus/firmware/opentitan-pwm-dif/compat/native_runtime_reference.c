#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

typedef struct gcc_mmio_event {
  uint32_t operation;
  uint32_t offset;
  uint32_t value;
} gcc_mmio_event_t;

enum {
  kGccMaximumEvents = 32,
};

extern volatile gcc_mmio_event_t gcc_mmio_events[kGccMaximumEvents];
extern volatile uint32_t gcc_mmio_event_count;
extern volatile uint32_t gcc_pwm_enable_register;
extern volatile uint32_t gcc_pwm_invert_register;
extern uint32_t gcc_firmware_entry(uint32_t runtime_channel);

int main(void) {
  for (uint32_t input = 0; input <= UINT8_MAX; ++input) {
    gcc_mmio_event_count = 0;
    gcc_pwm_enable_register = 0;
    gcc_pwm_invert_register = 0;
    uint32_t result = gcc_firmware_entry(input);
    uint32_t count = gcc_mmio_event_count;
    if (count > kGccMaximumEvents) {
      return 2;
    }
    printf("input_behavior=%" PRIu32 ",%" PRIu32 ",%" PRIu32 "\n", input,
           result, count);
    for (uint32_t event = 0; event < count; ++event) {
      printf("input_event=%" PRIu32 ",%" PRIu32 ",%" PRIu32 ",%" PRIu32
             ",%" PRIu32 "\n",
             input, event, gcc_mmio_events[event].operation,
             gcc_mmio_events[event].offset, gcc_mmio_events[event].value);
    }
  }
  return 0;
}
