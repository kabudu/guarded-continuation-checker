#ifndef GCC_OPENSBI_CONSOLE_H
#define GCC_OPENSBI_CONSOLE_H

struct sbi_console_device {
  const char *name;
  void (*console_putc)(char);
  int (*console_getc)(void);
};

static inline void sbi_console_set_device(struct sbi_console_device *device) {
  (void)device;
}

#endif
