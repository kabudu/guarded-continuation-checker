s/module aon_timer_core import aon_timer_reg_pkg::\*; (/module aon_timer_core (/
s/input  aon_timer_reg2hw_t         reg2hw_i/input  aon_timer_reg_pkg::aon_timer_reg2hw_t reg2hw_i/
s/lc_ctrl_pkg::lc_tx_test_false_strict(lc_escalate_en_i\[0\])/(lc_escalate_en_i[0] == 4'b1010)/
s/lc_ctrl_pkg::lc_tx_test_false_strict(lc_escalate_en_i\[1\])/(lc_escalate_en_i[1] == 4'b1010)/
s/lc_ctrl_pkg::lc_tx_test_false_strict(lc_escalate_en_i\[2\])/(lc_escalate_en_i[2] == 4'b1010)/
