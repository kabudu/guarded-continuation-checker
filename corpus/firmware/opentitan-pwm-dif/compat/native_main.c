#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

typedef struct gcc_mmio_event {
  uint32_t operation;
  uint32_t offset;
  uint32_t value;
} gcc_mmio_event_t;

extern volatile gcc_mmio_event_t gcc_mmio_events[32];
extern volatile uint32_t gcc_mmio_event_count;
uint32_t gcc_firmware_entry(void);

int main(void) {
  uint32_t result = gcc_firmware_entry();
  printf("firmware_result=%" PRIu32 "\n", result);
  printf("event_count=%" PRIu32 "\n", gcc_mmio_event_count);
  for (uint32_t index = 0; index < gcc_mmio_event_count; ++index) {
    printf("event=%" PRIu32 ",%" PRIu32 ",%" PRIu32 ",%" PRIu32 "\n",
           index, gcc_mmio_events[index].operation,
           gcc_mmio_events[index].offset, gcc_mmio_events[index].value);
  }
  printf("status=complete\n");
  return result == 0 ? 0 : 1;
}
