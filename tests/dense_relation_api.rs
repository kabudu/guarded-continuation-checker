use guarded_continuation_checker::dense_relation::DenseRelation;

#[test]
fn downstream_api_composes_and_powers_exact_nondeterministic_relations() {
    let mut step = DenseRelation::empty(4).unwrap();
    step.insert(0, 1).unwrap();
    step.insert(0, 2).unwrap();
    step.insert(1, 3).unwrap();
    step.insert(2, 3).unwrap();
    step.insert(3, 3).unwrap();

    let composed = DenseRelation::compose(&step, &step).unwrap();
    assert_eq!(composed.targets(0).unwrap(), [3]);
    assert_eq!(DenseRelation::power(&step, 2).unwrap(), composed);
    assert!(composed.contains(0, 3).unwrap());
    assert!(!composed.contains(0, 2).unwrap());
}
