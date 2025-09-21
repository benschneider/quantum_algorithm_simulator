use crate::core::types::BigEndian1Q;
use crate::core::endian_utils::build_controlled_gate_big_endian;
use nalgebra::{Complex, OMatrix, U2, U4, U8};
use num_traits::{One, Zero};
use std::f32::consts::FRAC_1_SQRT_2;

type ComplexF32 = Complex<f32>;

/// Builds the Hadamard gate matrix in big-endian ordering.
pub fn build_hadamard_big_endian() -> BigEndian1Q {
    let one_over_sqrt2 = ComplexF32::new(FRAC_1_SQRT_2, 0.0);
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        one_over_sqrt2, one_over_sqrt2,
        one_over_sqrt2, -one_over_sqrt2,
    ])
}

/// Builds the Pauli-X gate matrix in big-endian ordering.
pub fn pauli_x_big_endian() -> BigEndian1Q {
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        ComplexF32::zero(), ComplexF32::one(),
        ComplexF32::one(), ComplexF32::zero(),
    ])
}

/// Builds the Pauli-Y gate matrix in big-endian ordering.
pub fn pauli_y_big_endian() -> BigEndian1Q {
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        ComplexF32::zero(), -ComplexF32::i(),
        ComplexF32::i(), ComplexF32::zero(),
    ])
}

/// Builds the Pauli-Z gate matrix in big-endian ordering.
pub fn pauli_z_big_endian() -> BigEndian1Q {
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        ComplexF32::one(), ComplexF32::zero(),
        ComplexF32::zero(), -ComplexF32::one(),
    ])
}

/// Builds the square root of X gate matrix in big-endian ordering.
pub fn sqrt_x_big_endian() -> BigEndian1Q {
    let half = ComplexF32::new(0.5, 0.0);
    let i_half = ComplexF32::i() * half;
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        half, half - i_half,
        half - i_half, half,
    ])
}

/// Builds the square root of Y gate matrix in big-endian ordering.
pub fn sqrt_y_big_endian() -> BigEndian1Q {
    let half = 0.5;
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        half + ComplexF32::i() * half, -half - ComplexF32::i() * half,
        half + ComplexF32::i() * half, half - ComplexF32::i() * half,
    ])
}

/// Builds the square root of Z (S gate) matrix in big-endian ordering.
pub fn sqrt_z_big_endian() -> BigEndian1Q {
    OMatrix::<ComplexF32, U2, U2>::from_row_slice(&[
        ComplexF32::one(), ComplexF32::zero(),
        ComplexF32::zero(), ComplexF32::i(),
    ])
}

/// Builds the SWAP gate matrix in big-endian ordering.
pub fn swap_big_endian() -> crate::core::types::BigEndian2Q {
    OMatrix::<ComplexF32, U4, U4>::from_row_slice(&[
        ComplexF32::one(), ComplexF32::zero(), ComplexF32::zero(), ComplexF32::zero(),
        ComplexF32::zero(), ComplexF32::zero(), ComplexF32::one(), ComplexF32::zero(),
        ComplexF32::zero(), ComplexF32::one(), ComplexF32::zero(), ComplexF32::zero(),
        ComplexF32::zero(), ComplexF32::zero(), ComplexF32::zero(), ComplexF32::one(),
    ])
}

/// Builds the controlled-NOT (CX) gate matrix in big-endian ordering.
pub fn build_cx_big_endian() -> crate::core::types::BigEndian2Q {
    build_controlled_gate_big_endian(&pauli_x_big_endian())
}

/// Builds the controlled-Z (CZ) gate matrix in big-endian ordering.
pub fn build_cz_big_endian() -> crate::core::types::BigEndian2Q {
    build_controlled_gate_big_endian(&pauli_z_big_endian())
}

/// Builds the controlled-Y (CY) gate matrix in big-endian ordering.
pub fn build_cy_big_endian() -> crate::core::types::BigEndian2Q {
    build_controlled_gate_big_endian(&pauli_y_big_endian())
}

/// Builds the Toffoli (CCNOT) gate matrix in big-endian ordering.
pub fn build_ccnot_big_endian() -> crate::core::types::BigEndian3Q {
    let mut mat = OMatrix::<ComplexF32, U8, U8>::identity();
    // Flip |110> and |111>
    mat[(6, 7)] = ComplexF32::one();
    mat[(7, 6)] = ComplexF32::one();
    mat[(6, 6)] = ComplexF32::zero();
    mat[(7, 7)] = ComplexF32::zero();
    mat
}

/// Builds the controlled-controlled-Z (CCZ) gate matrix in big-endian ordering.
pub fn build_ccz_big_endian() -> crate::core::types::BigEndian3Q {
    let mut mat = OMatrix::<ComplexF32, U8, U8>::identity();
    // Flip sign of |111>
    mat[(7, 7)] = -ComplexF32::one();
    mat
}

/// Builds the Grover oracle matrix in big-endian ordering (marks |100>).
pub fn build_oracle_big_endian() -> crate::core::types::BigEndian3Q {
    let mut mat = OMatrix::<ComplexF32, U8, U8>::identity();
    // Mark |100> (index 4 in big-endian)
    mat[(4, 4)] = -ComplexF32::one();
    mat
}