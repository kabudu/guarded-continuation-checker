#ifndef GCC_OPENSBI_UART8250_H
#define GCC_OPENSBI_UART8250_H

#include <stdint.h>

typedef uint16_t u16;
typedef uint32_t u32;

#define UART_CAP_UUE (1U << 0)

struct uart8250_device {
  volatile char *base;
  u32 reg_shift;
  u32 reg_width;
  u32 in_freq;
  u32 baudrate;
};

void uart8250_device_putc(struct uart8250_device *dev, char ch);
int uart8250_device_getc(struct uart8250_device *dev);
void uart8250_device_init(struct uart8250_device *dev, unsigned long base,
                          u32 in_freq, u32 baudrate, u32 reg_shift,
                          u32 reg_width, u32 reg_offset, u32 caps);
int uart8250_init(unsigned long base, u32 in_freq, u32 baudrate, u32 reg_shift,
                  u32 reg_width, u32 reg_offset, u32 caps);

#endif
