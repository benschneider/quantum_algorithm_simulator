use super::matrix_builders::{build_cx_big_endian, build_cy_big_endian, build_cz_big_endian};
use super::parametric::{
    eval_cx_pow_big, eval_cy_pow_big, eval_cz_pow_big, eval_rx_big, eval_ry_big, eval_rz_big,
};
use super::registry::GateRegistry;
use crate::core::types::{Gate, Param};
use nalgebra::{Complex, DMatrix, DVector};
use std::f32::consts::{FRAC_1_SQRT_2, PI};

const TOL: f32 = 1e-5;

fn c(re: f32, im: f32) -> Complex<f32> {
    Complex::new(re, im)
}

fn to_dmatrix<const R: usize, const C: usize, S>(matrix: &nalgebra::Matrix<Complex<f32>, nalgebra::Const<R>, nalgebra::Const<C>, S>) -> DMatrix<Complex<f32>>
where
    S: nalgebra::storage::Storage<Complex<f32>, nalgebra::Const<R>, nalgebra::Const<C>>,
{
    DMatrix::from_fn(R, C, |r, c| matrix[(r, c)])
}

fn ket(amplitudes: &[Complex<f32>]) -> DVector<Complex<f32>> {
    DVector::from_column_slice(amplitudes)
}

fn basis(index: usize, dimension: usize) -> DVector<Complex<f32>> {
    let mut state = DVector::from_element(dimension, c(0.0, 0.0));
    state[index] = c(1.0, 0.0);
    state
}

fn infer_global_phase(actual: &[Complex<f32>], expected: &[Complex<f32>]) -> Complex<f32> {
    for (a, e) in actual.iter().zip(expected.iter()) {
        if e.norm() > TOL && a.norm() > TOL {
            let ratio = *a / *e;
            return ratio / c(ratio.norm(), 0.0);
        }
    }
    c(1.0, 0.0)
}

fn assert_vectors_close(actual: &DVector<Complex<f32>>, expected: &DVector<Complex<f32>>) {
    assert_eq!(actual.len(), expected.len());
    for i in 0..actual.len() {
        let diff = actual[i] - expected[i];
        assert!(
            diff.norm() < TOL,
            "vector mismatch at index {i}: actual={:?}, expected={:?}",
            actual[i],
            expected[i]
        );
    }
}

fn assert_matrices_close(actual: &DMatrix<Complex<f32>>, expected: &DMatrix<Complex<f32>>) {
    assert_eq!(actual.shape(), expected.shape());
    for r in 0..actual.nrows() {
        for c_idx in 0..actual.ncols() {
            let diff = actual[(r, c_idx)] - expected[(r, c_idx)];
            assert!(
                diff.norm() < TOL,
                "matrix mismatch at ({r}, {c_idx}): actual={:?}, expected={:?}",
                actual[(r, c_idx)],
                expected[(r, c_idx)]
            );
        }
    }
}

fn assert_matrices_close_up_to_global_phase(
    actual: &DMatrix<Complex<f32>>,
    expected: &DMatrix<Complex<f32>>,
) {
    assert_eq!(actual.shape(), expected.shape());
    let actual_flat: Vec<_> = actual.iter().copied().collect();
    let expected_flat: Vec<_> = expected.iter().copied().collect();
    let phase = infer_global_phase(&actual_flat, &expected_flat);

    for r in 0..actual.nrows() {
        for c_idx in 0..actual.ncols() {
            let diff = actual[(r, c_idx)] - phase * expected[(r, c_idx)];
            assert!(
                diff.norm() < TOL,
                "matrix mismatch up to phase at ({r}, {c_idx}): actual={:?}, expected={:?}, phase={:?}",
                actual[(r, c_idx)],
                expected[(r, c_idx)],
                phase
            );
        }
    }
}

fn assert_vector_close_up_to_global_phase(
    actual: &DVector<Complex<f32>>,
    expected: &DVector<Complex<f32>>,
) {
    assert_eq!(actual.len(), expected.len());
    let actual_flat: Vec<_> = actual.iter().copied().collect();
    let expected_flat: Vec<_> = expected.iter().copied().collect();
    let phase = infer_global_phase(&actual_flat, &expected_flat);

    for i in 0..actual.len() {
        let diff = actual[i] - phase * expected[i];
        assert!(
            diff.norm() < TOL,
            "vector mismatch up to phase at index {i}: actual={:?}, expected={:?}, phase={:?}",
            actual[i],
            expected[i],
            phase
        );
    }
}

fn registry_matrix(registry: &GateRegistry, gate: Gate, params: &[f32], qubits: &[u32]) -> DMatrix<Complex<f32>> {
    registry
        .eval(
            &gate,
            &params.iter().copied().map(Param::Scalar).collect::<Vec<_>>(),
            qubits,
        )
        .expect("gate should exist")
        .to_dmatrix()
}

#[test]
fn eval_rx_big_matches_standard_actions() {
    let rx_pi = to_dmatrix(&eval_rx_big(&[Param::Scalar(PI)]));
    let rx_half_pi = to_dmatrix(&eval_rx_big(&[Param::Scalar(PI / 2.0)]));
    let zero = basis(0, 2);

    let actual_pi = &rx_pi * &zero;
    let expected_pi = ket(&[c(0.0, 0.0), c(0.0, -1.0)]);
    assert_vectors_close(&actual_pi, &expected_pi);

    let actual_half_pi = &rx_half_pi * zero;
    let expected_half_pi = ket(&[c(FRAC_1_SQRT_2, 0.0), c(0.0, -FRAC_1_SQRT_2)]);
    assert_vectors_close(&actual_half_pi, &expected_half_pi);
}

#[test]
fn eval_ry_big_matches_standard_actions() {
    let ry_pi = to_dmatrix(&eval_ry_big(&[Param::Scalar(PI)]));
    let ry_half_pi = to_dmatrix(&eval_ry_big(&[Param::Scalar(PI / 2.0)]));
    let zero = basis(0, 2);

    let actual_pi = &ry_pi * &zero;
    let expected_pi = ket(&[c(0.0, 0.0), c(1.0, 0.0)]);
    assert_vectors_close(&actual_pi, &expected_pi);

    let actual_half_pi = &ry_half_pi * zero;
    let expected_half_pi = ket(&[c(FRAC_1_SQRT_2, 0.0), c(FRAC_1_SQRT_2, 0.0)]);
    assert_vectors_close(&actual_half_pi, &expected_half_pi);
}

#[test]
fn eval_rz_big_matches_standard_basis_action_and_composition() {
    let rz_pi = to_dmatrix(&eval_rz_big(&[Param::Scalar(PI)]));
    let zero = basis(0, 2);
    let one = basis(1, 2);

    let actual_zero = &rz_pi * &zero;
    let expected_zero = ket(&[c(0.0, -1.0), c(0.0, 0.0)]);
    assert_vectors_close(&actual_zero, &expected_zero);

    let actual_one = &rz_pi * &one;
    let expected_one = ket(&[c(0.0, 0.0), c(0.0, 1.0)]);
    assert_vectors_close(&actual_one, &expected_one);

    let a = 0.37;
    let b = -0.91;
    let composed = to_dmatrix(&eval_rz_big(&[Param::Scalar(a)]))
        * to_dmatrix(&eval_rz_big(&[Param::Scalar(b)]));
    let combined = to_dmatrix(&eval_rz_big(&[Param::Scalar(a + b)]));
    assert_matrices_close(&composed, &combined);
}

#[test]
fn single_qubit_rotations_are_invertible() {
    for angle in [0.1, -0.3, 1.2] {
        let rx = to_dmatrix(&eval_rx_big(&[Param::Scalar(angle)]));
        let rx_inv = to_dmatrix(&eval_rx_big(&[Param::Scalar(-angle)]));
        assert_matrices_close(&(rx * rx_inv), &DMatrix::identity(2, 2));

        let ry = to_dmatrix(&eval_ry_big(&[Param::Scalar(angle)]));
        let ry_inv = to_dmatrix(&eval_ry_big(&[Param::Scalar(-angle)]));
        assert_matrices_close(&(ry * ry_inv), &DMatrix::identity(2, 2));

        let rz = to_dmatrix(&eval_rz_big(&[Param::Scalar(angle)]));
        let rz_inv = to_dmatrix(&eval_rz_big(&[Param::Scalar(-angle)]));
        assert_matrices_close(&(rz * rz_inv), &DMatrix::identity(2, 2));
    }
}

#[test]
fn full_cz_maps_big_endian_plus_plus_state_correctly() {
    // Big-endian basis ordering is |00>, |01>, |10>, |11>.
    let plus_plus = ket(&[
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
    ]);
    let expected = ket(&[
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(-0.5, 0.0),
    ]);

    let cz = to_dmatrix(&build_cz_big_endian());
    let actual = cz * plus_plus;
    assert_vectors_close(&actual, &expected);
}

#[test]
fn raw_controlled_pow_builders_match_full_gates_at_exponent_one() {
    let cx_pow = to_dmatrix(&eval_cx_pow_big(&[Param::Scalar(1.0)]));
    let cy_pow = to_dmatrix(&eval_cy_pow_big(&[Param::Scalar(1.0)]));
    let cz_pow = to_dmatrix(&eval_cz_pow_big(&[Param::Scalar(1.0)]));

    assert_matrices_close_up_to_global_phase(&cx_pow, &to_dmatrix(&build_cx_big_endian()));
    assert_matrices_close_up_to_global_phase(&cy_pow, &to_dmatrix(&build_cy_big_endian()));
    assert_matrices_close_up_to_global_phase(&cz_pow, &to_dmatrix(&build_cz_big_endian()));
}

#[test]
fn registry_rotations_use_radian_semantics() {
    let registry = GateRegistry::new_with_standard_gates();

    let rx = registry_matrix(&registry, Gate::Rx, &[PI], &[0]);
    let expected_rx = to_dmatrix(&eval_rx_big(&[Param::Scalar(PI)]));
    assert_matrices_close_up_to_global_phase(&rx, &expected_rx);

    let ry = registry_matrix(&registry, Gate::Ry, &[PI / 2.0], &[0]);
    let expected_ry = to_dmatrix(&eval_ry_big(&[Param::Scalar(PI / 2.0)]));
    assert_matrices_close_up_to_global_phase(&ry, &expected_ry);

    let rz = registry_matrix(&registry, Gate::Rz, &[PI], &[0]);
    let expected_rz = to_dmatrix(&eval_rz_big(&[Param::Scalar(PI)]));
    assert_matrices_close_up_to_global_phase(&rz, &expected_rz);
}

#[test]
fn registry_pow_gates_use_full_gate_exponent_semantics() {
    let registry = GateRegistry::new_with_standard_gates();

    let cx_pow = registry_matrix(&registry, Gate::CXPow, &[1.0], &[0, 1]);
    let cy_pow = registry_matrix(&registry, Gate::CYPow, &[1.0], &[0, 1]);
    let cz_pow = registry_matrix(&registry, Gate::CZPow, &[1.0], &[0, 1]);

    assert_matrices_close_up_to_global_phase(&cx_pow, &registry_matrix(&registry, Gate::CX, &[], &[0, 1]));
    assert_matrices_close_up_to_global_phase(&cy_pow, &registry_matrix(&registry, Gate::CY, &[], &[0, 1]));
    assert_matrices_close_up_to_global_phase(&cz_pow, &registry_matrix(&registry, Gate::CZ, &[], &[0, 1]));
}

#[test]
fn registry_pow_gates_match_cirq_style_single_qubit_relations_up_to_global_phase() {
    let registry = GateRegistry::new_with_standard_gates();

    let x_pow_half = registry_matrix(&registry, Gate::XPow, &[0.5], &[0]);
    let rx_half_pi = to_dmatrix(&eval_rx_big(&[Param::Scalar(PI / 2.0)]));
    assert_matrices_close_up_to_global_phase(&x_pow_half, &rx_half_pi);

    let y_pow_half = registry_matrix(&registry, Gate::YPow, &[0.5], &[0]);
    let ry_half_pi = to_dmatrix(&eval_ry_big(&[Param::Scalar(PI / 2.0)]));
    assert_matrices_close_up_to_global_phase(&y_pow_half, &ry_half_pi);

    let z_pow_one = registry_matrix(&registry, Gate::ZPow, &[1.0], &[0]);
    let rz_pi = to_dmatrix(&eval_rz_big(&[Param::Scalar(PI)]));
    assert_matrices_close_up_to_global_phase(&z_pow_one, &rz_pi);
}

#[test]
fn registry_czpow_full_gate_has_expected_phase_action() {
    let registry = GateRegistry::new_with_standard_gates();
    let cz_pow = registry_matrix(&registry, Gate::CZPow, &[1.0], &[0, 1]);
    let plus_plus = ket(&[
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
    ]);
    let expected = ket(&[
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(0.5, 0.0),
        c(-0.5, 0.0),
    ]);

    let actual = cz_pow * plus_plus;
    assert_vector_close_up_to_global_phase(&actual, &expected);
}

#[test]
fn registry_move_gate_matches_logical_state_transfer_action() {
    let registry = GateRegistry::new_with_standard_gates();
    let move_gate = registry_matrix(&registry, Gate::MOVE, &[], &[0, 1]);

    let zero_one = basis(1, 4);
    let one_zero = basis(2, 4);
    assert_vectors_close(&(move_gate.clone() * zero_one), &basis(2, 4));
    assert_vectors_close(&(move_gate * one_zero), &basis(1, 4));
}
