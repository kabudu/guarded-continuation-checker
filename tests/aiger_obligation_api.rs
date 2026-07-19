use guarded_continuation_checker::aiger_obligation::{
    AigerInputPredicate, AigerLatch, AigerTransition, relation_row_completeness_cnf,
    terminal_completeness_cnf,
};
use guarded_continuation_checker::unsat_proof::{generate_unsat_proof, verify_unsat_proof};

fn toggling_controller() -> AigerTransition {
    AigerTransition {
        max_variable: 1,
        inputs: vec![],
        latches: vec![AigerLatch {
            current: 2,
            next: 3,
        }],
        outputs: vec![2],
        ands: vec![],
    }
}

#[test]
fn downstream_api_proves_relation_and_terminal_completeness() {
    let controller = toggling_controller();
    let predicate = AigerInputPredicate { clauses: vec![] };

    let row = relation_row_completeness_cnf(&controller, &[], 0, &predicate, &[1]).unwrap();
    let row_proof = generate_unsat_proof(&row).unwrap();
    verify_unsat_proof(&row, &row_proof).unwrap();

    let terminal = terminal_completeness_cnf(&controller, &[], &predicate, 0, &[0]).unwrap();
    let terminal_proof = generate_unsat_proof(&terminal).unwrap();
    verify_unsat_proof(&terminal, &terminal_proof).unwrap();

    assert_eq!(controller.evaluate(0, 0).unwrap(), (1, 0));
    assert_eq!(controller.evaluate(1, 0).unwrap(), (0, 1));
}
