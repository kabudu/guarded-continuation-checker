#include <stdint.h>

#include "upstream/uart8250.c"

typedef struct gcc_mmio_event {
  uint32_t operation;
  uint32_t offset;
  uint32_t value;
} gcc_mmio_event_t;

enum {
  kGccUartState = 5,
  kGccUartReceive = 6,
  kGccMaximumEvents = 32,
  kUartLineStatus = 5,
  kUartTransmitterReady = 0x20,
  kUartDataReady = 0x01,
};

volatile gcc_mmio_event_t gcc_mmio_events[kGccMaximumEvents];
volatile uint32_t gcc_mmio_event_count;
static volatile uint8_t gcc_uart_registers[128] __attribute__((aligned(4)));
static struct uart8250_device gcc_uart_device;

static void record_event(uint32_t operation, uint32_t offset, uint32_t value) {
  uint32_t index = gcc_mmio_event_count;
  if (index < kGccMaximumEvents) {
    gcc_mmio_events[index].operation = operation;
    gcc_mmio_events[index].offset = offset;
    gcc_mmio_events[index].value = value;
    gcc_mmio_event_count = index + 1;
  }
}

static uint32_t select_width(uint32_t input) {
  switch (input & 3U) {
    case 0:
      return 1;
    case 1:
      return 2;
    default:
      return 4;
  }
}

static uint32_t select_shift(uint32_t input, uint32_t width) {
  if (width == 4) {
    return 2;
  }
  if (width == 2) {
    return (input & 4U) ? 2 : 1;
  }
  switch ((input >> 2) & 3U) {
    case 0:
      return 0;
    case 1:
      return 1;
    default:
      return 2;
  }
}

static void seed_register(uint32_t shift, uint32_t width, uint32_t number,
                          uint32_t value) {
  volatile void *address = &gcc_uart_registers[number << shift];
  if (width == 1) {
    writeb(value, address);
  } else if (width == 2) {
    writew(value, address);
  } else {
    writel(value, address);
  }
}

__attribute__((used)) uint32_t gcc_firmware_entry(uint32_t input) {
  uint32_t width = select_width(input);
  uint32_t shift = select_shift(input, width);
  uint32_t baudrate = (input & 0x10U) ? 115200U : 0U;
  uint32_t caps = (input & 0x20U) ? UART_CAP_UUE : 0U;
  uint32_t status = kUartTransmitterReady;
  int received;

  if ((input & 0x40U) != 0) {
    status |= kUartDataReady;
  }
  seed_register(shift, width, 0, input ^ 0xa5U);
  seed_register(shift, width, kUartLineStatus, status);

  uart8250_device_init(&gcc_uart_device,
                       (unsigned long)&gcc_uart_registers[0], 24000000U,
                       baudrate, shift, width, 0, caps);
  received = uart8250_device_getc(&gcc_uart_device);
  uart8250_device_putc(&gcc_uart_device, (char)(input ^ 0x5aU));

  for (uint32_t number = 0; number <= 8; number++) {
    record_event(kGccUartState, number, get_reg(&gcc_uart_device, number));
  }
  record_event(kGccUartReceive, 0, (uint32_t)received);
  return width | (shift << 8);
}
