#ifndef GCC_OPENSBI_DOMAIN_H
#define GCC_OPENSBI_DOMAIN_H

#define SBI_DOMAIN_MEMREGION_MMIO (1UL << 0)
#define SBI_DOMAIN_MEMREGION_SHARED_SURW_MRW (1UL << 1)

static inline int sbi_domain_root_add_memrange(unsigned long base,
                                               unsigned long size,
                                               unsigned long alignment,
                                               unsigned long flags) {
  (void)base;
  (void)size;
  (void)alignment;
  (void)flags;
  return 0;
}

#endif
