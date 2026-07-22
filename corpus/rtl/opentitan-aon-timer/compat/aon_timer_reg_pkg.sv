package aon_timer_reg_pkg;
  typedef struct packed { logic q; logic qe; } alert_test_t;
  typedef struct packed { logic [11:0] q; } prescaler_t;
  typedef struct packed { logic q; } bit_q_t;
  typedef struct packed { prescaler_t prescaler; bit_q_t enable; } wkup_ctrl_t;
  typedef struct packed { logic [31:0] q; } word_q_t;
  typedef struct packed { bit_q_t pause_in_sleep; bit_q_t enable; } wdog_ctrl_t;
  typedef struct packed { bit_q_t wdog_timer_bark; bit_q_t wkup_timer_expired; } intr_state_t;
  typedef struct packed { alert_test_t wdog_timer_bark; alert_test_t wkup_timer_expired; } intr_test_t;
  typedef struct packed {
    alert_test_t alert_test;
    wkup_ctrl_t wkup_ctrl;
    word_q_t wkup_thold_hi;
    word_q_t wkup_thold_lo;
    word_q_t wkup_count_hi;
    word_q_t wkup_count_lo;
    wdog_ctrl_t wdog_ctrl;
    word_q_t wdog_bark_thold;
    word_q_t wdog_bite_thold;
    word_q_t wdog_count;
    intr_state_t intr_state;
    intr_test_t intr_test;
    bit_q_t wkup_cause;
  } aon_timer_reg2hw_t;
endpackage
