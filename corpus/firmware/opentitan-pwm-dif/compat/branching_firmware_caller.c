#include <stdint.h>

#ifndef GCC_BRANCH_CLASSES
#error "GCC_BRANCH_CLASSES must be defined"
#endif

#if GCC_BRANCH_CLASSES != 1 && GCC_BRANCH_CLASSES != 2 && \
    GCC_BRANCH_CLASSES != 4 && GCC_BRANCH_CLASSES != 8 && \
    GCC_BRANCH_CLASSES != 16 && GCC_BRANCH_CLASSES != 32
#error "GCC_BRANCH_CLASSES must be a supported power of two"
#endif

#define GCC_RUNTIME_CHANNEL_CONTROL
#define gcc_firmware_entry gcc_branching_base_entry
#include "firmware_caller.c"
#undef gcc_firmware_entry

#define GCC_BRANCH_WRAPPER(index)                                      \
  __attribute__((noinline, used)) static uint32_t gcc_branch_##index(  \
      uint32_t runtime_channel) {                                      \
    __asm__ volatile("" : : "r"(runtime_channel) : "memory");          \
    return gcc_branching_base_entry(runtime_channel);                  \
  }

GCC_BRANCH_WRAPPER(0)
GCC_BRANCH_WRAPPER(1)
GCC_BRANCH_WRAPPER(2)
GCC_BRANCH_WRAPPER(3)
GCC_BRANCH_WRAPPER(4)
GCC_BRANCH_WRAPPER(5)
GCC_BRANCH_WRAPPER(6)
GCC_BRANCH_WRAPPER(7)
GCC_BRANCH_WRAPPER(8)
GCC_BRANCH_WRAPPER(9)
GCC_BRANCH_WRAPPER(10)
GCC_BRANCH_WRAPPER(11)
GCC_BRANCH_WRAPPER(12)
GCC_BRANCH_WRAPPER(13)
GCC_BRANCH_WRAPPER(14)
GCC_BRANCH_WRAPPER(15)
GCC_BRANCH_WRAPPER(16)
GCC_BRANCH_WRAPPER(17)
GCC_BRANCH_WRAPPER(18)
GCC_BRANCH_WRAPPER(19)
GCC_BRANCH_WRAPPER(20)
GCC_BRANCH_WRAPPER(21)
GCC_BRANCH_WRAPPER(22)
GCC_BRANCH_WRAPPER(23)
GCC_BRANCH_WRAPPER(24)
GCC_BRANCH_WRAPPER(25)
GCC_BRANCH_WRAPPER(26)
GCC_BRANCH_WRAPPER(27)
GCC_BRANCH_WRAPPER(28)
GCC_BRANCH_WRAPPER(29)
GCC_BRANCH_WRAPPER(30)
GCC_BRANCH_WRAPPER(31)

__attribute__((used)) uint32_t gcc_firmware_entry(uint32_t runtime_channel) {
  if (runtime_channel < 6) {
    return gcc_branching_base_entry(runtime_channel);
  }
  switch ((runtime_channel - 6) & (GCC_BRANCH_CLASSES - 1)) {
    case 0:
      return gcc_branch_0(runtime_channel);
    case 1:
      return gcc_branch_1(runtime_channel);
    case 2:
      return gcc_branch_2(runtime_channel);
    case 3:
      return gcc_branch_3(runtime_channel);
    case 4:
      return gcc_branch_4(runtime_channel);
    case 5:
      return gcc_branch_5(runtime_channel);
    case 6:
      return gcc_branch_6(runtime_channel);
    case 7:
      return gcc_branch_7(runtime_channel);
    case 8:
      return gcc_branch_8(runtime_channel);
    case 9:
      return gcc_branch_9(runtime_channel);
    case 10:
      return gcc_branch_10(runtime_channel);
    case 11:
      return gcc_branch_11(runtime_channel);
    case 12:
      return gcc_branch_12(runtime_channel);
    case 13:
      return gcc_branch_13(runtime_channel);
    case 14:
      return gcc_branch_14(runtime_channel);
    case 15:
      return gcc_branch_15(runtime_channel);
    case 16:
      return gcc_branch_16(runtime_channel);
    case 17:
      return gcc_branch_17(runtime_channel);
    case 18:
      return gcc_branch_18(runtime_channel);
    case 19:
      return gcc_branch_19(runtime_channel);
    case 20:
      return gcc_branch_20(runtime_channel);
    case 21:
      return gcc_branch_21(runtime_channel);
    case 22:
      return gcc_branch_22(runtime_channel);
    case 23:
      return gcc_branch_23(runtime_channel);
    case 24:
      return gcc_branch_24(runtime_channel);
    case 25:
      return gcc_branch_25(runtime_channel);
    case 26:
      return gcc_branch_26(runtime_channel);
    case 27:
      return gcc_branch_27(runtime_channel);
    case 28:
      return gcc_branch_28(runtime_channel);
    case 29:
      return gcc_branch_29(runtime_channel);
    case 30:
      return gcc_branch_30(runtime_channel);
    default:
      return gcc_branch_31(runtime_channel);
  }
}
