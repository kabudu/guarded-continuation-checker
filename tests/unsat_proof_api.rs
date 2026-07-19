use guarded_continuation_checker::unsat_proof::{
    CnfClause, generate_unsat_proof, verify_unsat_proof,
};

#[test]
fn downstream_api_generates_and_independently_checks_unsat_evidence() {
    let clauses = vec![
        CnfClause(vec![(0, true)]),
        CnfClause(vec![(0, false), (1, true)]),
        CnfClause(vec![(1, false)]),
    ];
    let proof = generate_unsat_proof(&clauses).unwrap();
    verify_unsat_proof(&clauses, &proof).unwrap();

    let mut corrupted = proof;
    corrupted[0] ^= 1;
    assert!(verify_unsat_proof(&clauses, &corrupted).is_err());
}
