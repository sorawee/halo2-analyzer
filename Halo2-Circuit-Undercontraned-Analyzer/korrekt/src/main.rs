

use halo2_proofs::dev::MockProver;
use halo2_proofs::pasta::Fp as Fr;

mod abstract_expr;
mod layouter;
mod shape;

mod sample_circuits;
mod analyzer;

fn main() {
    println!("----------------------Circuit----------------------");
    let circuit = sample_circuits::PlayCircuit::<Fr>::new(Fr::from(1), Fr::from(1));
    let mut analyzer = analyzer::Analyzer::new_with(&circuit);
    let k = 5;

    let public_input = Fr::from(3);
    //mockprover verify passes
    let prover = MockProver::<Fr>::run(k, &circuit, vec![vec![public_input]]).unwrap();
    prover.verify().expect("verify should work");
    analyzer.analyze_underconstrained();

    // println!("----------------------Multi Circuit----------------------");
    // let multi_circuit = MultiPlayCircuit::<Fr>::new(Fr::from(1), Fr::from(1));
    // let mut analyzer1 = Analyzer::new_with(&multi_circuit);

    // let k = 5;

    // let public_input1 = Fr::from(3);
    // log::debug!("running mock prover...");
    // let prover1 = MockProver::<Fr>::run(k, &multi_circuit, vec![vec![public_input1]]).unwrap();

    // prover1.verify().expect("verify should work");
    // log::debug!("verified via mock prover...");

    // analyzer1.analyze_underconstrained();
}

#[cfg(test)]
mod integration_tests;
