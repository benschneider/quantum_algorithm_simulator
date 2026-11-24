//! Double-precision reference evolution for the patch experiment.
//! Wire q is bit q of the dense basis index. Every amplitude is retained.
use nalgebra::DMatrix;
use num_complex::Complex64;
use quantsim_core::types::{Gate, Operation};
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_6};

/// Ideal matrices in the local basis |ab⟩, with the first operand as a.
pub fn ideal_matrix(gate: &Gate) -> DMatrix<Complex64> {
    let zero = Complex64::ZERO;
    let one = Complex64::ONE;
    let h = Complex64::from(FRAC_1_SQRT_2);
    let plus = Complex64::new(0.5, 0.5);
    let minus = plus.conj();
    match gate {
        Gate::H => DMatrix::from_row_slice(2, 2, &[h, h, h, -h]),
        Gate::X => DMatrix::from_row_slice(2, 2, &[zero, one, one, zero]),
        Gate::SqrtX => DMatrix::from_row_slice(2, 2, &[plus, minus, minus, plus]),
        Gate::SqrtY => DMatrix::from_row_slice(2, 2, &[plus, -plus, plus, plus]),
        Gate::SYC => {
            let mut matrix = DMatrix::zeros(4, 4);
            matrix[(0, 0)] = one;
            matrix[(1, 2)] = -Complex64::I;
            matrix[(2, 1)] = -Complex64::I;
            matrix[(3, 3)] = Complex64::from_polar(1.0, -FRAC_PI_6);
            matrix
        }
        other => panic!("unsupported patch gate {other}"),
    }
}

pub struct DenseOracle {
    pub amplitudes: Vec<Complex64>,
}

impl DenseOracle {
    pub fn new(wires: usize) -> Self {
        assert!(
            wires <= 16,
            "dense reference is limited to the 16-wire patch"
        );
        let mut amplitudes = vec![Complex64::ZERO; 1 << wires];
        amplitudes[0] = Complex64::ONE;
        Self { amplitudes }
    }

    pub fn apply(&mut self, op: &Operation) {
        let wires = self.amplitudes.len().ilog2();
        assert!(op.qubits.iter().all(|&q| q < wires), "wire outside patch");
        let matrix = ideal_matrix(&op.id);
        match *op.qubits.as_slice() {
            [q] => self.apply_subspaces([0, 1 << q], &matrix),
            [a, b] if a != b => {
                let (a_bit, b_bit) = (1 << a, 1 << b);
                self.apply_subspaces([0, b_bit, a_bit, a_bit | b_bit], &matrix);
            }
            _ => panic!("invalid patch operands {:?}", op.qubits),
        }
    }

    fn apply_subspaces<const N: usize>(
        &mut self,
        offsets: [usize; N],
        matrix: &DMatrix<Complex64>,
    ) {
        assert_eq!(matrix.shape(), (N, N), "patch gate arity mismatch");
        let mask = offsets[N - 1];
        for base in 0..self.amplitudes.len() {
            if base & mask != 0 {
                continue;
            }
            let inputs = offsets.map(|offset| self.amplitudes[base | offset]);
            for (row, offset) in offsets.iter().enumerate() {
                self.amplitudes[base | offset] =
                    (0..N).map(|col| matrix[(row, col)] * inputs[col]).sum();
            }
        }
    }
}
