use nalgebra::{DMatrix, OMatrix, Complex, U2, U4};
use nalgebra_sparse::CsrMatrix;
use num_traits::One;
use std::error::Error;

type ComplexF32 = Complex<f32>;

/// Generates a bit reversal permutation for converting between big-endian and little-endian qubit ordering.
/// 
/// For n qubits, reverses the bit order of indices from 0 to 2^n - 1.
pub fn bit_reverse(n_qubits: usize) -> Vec<usize> {
    let n = 1usize << n_qubits;
    let log_n = n_qubits;
    let result: Vec<usize> = (0..n).map(|i| {
        let mut rev = 0usize;
        let mut x = i;
        for _ in 0..log_n {
            rev = (rev << 1) | (x & 1);
            x >>= 1;
        }
        rev
    }).collect();
    result
}

/// Converts a dense big-endian matrix to a sparse little-endian CSR matrix.
/// 
/// Validates that the matrix dimensions are a power of 2 (valid for qubit systems).
/// Performs bit reversal on row/column indices to convert from big-endian to little-endian ordering.
pub fn to_sparse_little_endian(matrix: &DMatrix<ComplexF32>) -> Result<CsrMatrix<ComplexF32>, Box<dyn Error>> {
    let (rows, cols) = matrix.shape();
    if rows != cols {
        return Err("Matrix must be square".into());
    }
    let n = rows as usize;
    if !n.is_power_of_two() {
        return Err(format!("Matrix dimension {} must be a power of 2 for qubit systems", n).into());
    }
    
    // Create a new dense matrix for the little-endian layout.
    let mut little_matrix = DMatrix::<ComplexF32>::zeros(rows, cols);
    let bit_reverse_map = bit_reverse(n.trailing_zeros() as usize);

    // Permute the big-endian matrix into the little-endian dense matrix.
    for r in 0..rows {
        for c in 0..cols {
            let rev_r = bit_reverse_map[r];
            let rev_c = bit_reverse_map[c];
            little_matrix[(rev_r, rev_c)] = matrix[(r, c)];
        }
    }

    // Convert the now-correct little-endian dense matrix to a sparse format.
    Ok(CsrMatrix::from(&little_matrix))
}

/// Builds a controlled gate matrix in big-endian ordering using the given single-qubit target gate.
/// 
/// For a control on MSB and target on next, constructs the block diagonal matrix:
/// [[I, 0], [0, U]] where U is the target gate matrix.
pub fn build_controlled_gate_big_endian(
    target_gate: &OMatrix<ComplexF32, U2, U2>,
) -> OMatrix<ComplexF32, U4, U4> {
    let mut controlled = OMatrix::<ComplexF32, U4, U4>::zeros();
    // Top-left: Identity 2x2
    controlled[(0, 0)] = ComplexF32::one();
    controlled[(1, 1)] = ComplexF32::one();
    // Bottom-right: target_gate (2x2) 
    controlled[(2, 2)] = target_gate[(0, 0)];
    controlled[(2, 3)] = target_gate[(0, 1)];
    controlled[(3, 2)] = target_gate[(1, 0)];
    controlled[(3, 3)] = target_gate[(1, 1)];
    controlled
}
/// Checks if a given matrix is unitary.
///
/// A matrix `U` is unitary if `U * U.adjoint() = I`, where `I` is the identity
/// matrix. This function returns a tuple `(bool, f32)` where the boolean is
/// true if the matrix is unitary, and the float is the Frobenius norm of the
/// deviation from identity.
pub fn is_unitary(matrix: &DMatrix<Complex<f32>>) -> (bool, f32) {
    let u_adj = matrix.adjoint();
    let product = matrix * u_adj;
    let identity = DMatrix::identity(product.nrows(), product.ncols());
    let diff = product - identity;
    let norm = diff.norm();
    (norm < 1e-6, norm)
}