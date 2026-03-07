use crate::core::endian_utils::build_controlled_gate_big_endian;
use crate::core::types::{BigEndian1Q, BigEndian2Q, Param};
use nalgebra::{Complex, OMatrix, U2};
use std::f32::consts::PI;

fn scalar_param(params: &[Param], default: f32) -> f32 {
    if let Some(Param::Scalar(value)) = params.first() {
        *value
    } else {
        default
    }
}

fn eval_x_pow_big(exponent: f32) -> BigEndian1Q {
    let angle = exponent * PI;
    let global_phase = (Complex::new(0.0, angle / 2.0)).exp();
    eval_rx_big(&[Param::Scalar(angle)]) * global_phase
}

fn eval_y_pow_big(exponent: f32) -> BigEndian1Q {
    let angle = exponent * PI;
    let global_phase = (Complex::new(0.0, angle / 2.0)).exp();
    eval_ry_big(&[Param::Scalar(angle)]) * global_phase
}

fn eval_z_pow_big(exponent: f32) -> BigEndian1Q {
    let phase = Complex::new((PI * exponent).cos(), (PI * exponent).sin());
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        Complex::new(1.0, 0.0), Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0), phase,
    ])
}

/// Builds the standard physics `Rz(theta)` matrix in big-endian ordering.
///
/// The parameter is a physical rotation angle in radians. In the single-qubit
/// big-endian basis `|0>, |1>`, the matrix is
/// `diag(exp(-i theta / 2), exp(+i theta / 2))`.
pub fn eval_rz_big(params: &[Param]) -> BigEndian1Q {
    let theta = scalar_param(params, 0.0);
    let minus = (Complex::new(0.0, -theta / 2.0)).exp();
    let plus = (Complex::new(0.0, theta / 2.0)).exp();
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        minus, Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0), plus,
    ])
}

/// Builds the standard physics `Rx(theta)` matrix in big-endian ordering.
///
/// The parameter is a physical rotation angle in radians. In the single-qubit
/// big-endian basis `|0>, |1>`, the matrix is
/// `cos(theta / 2) I - i sin(theta / 2) X`.
pub fn eval_rx_big(params: &[Param]) -> BigEndian1Q {
    let theta = scalar_param(params, 0.0);
    let (s, c) = (theta / 2.0).sin_cos();
    let i_s = Complex::new(0.0, -s);
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        Complex::new(c, 0.0), i_s,
        i_s, Complex::new(c, 0.0),
    ])
}

/// Builds the standard physics `Ry(theta)` matrix in big-endian ordering.
///
/// The parameter is a physical rotation angle in radians. In the single-qubit
/// big-endian basis `|0>, |1>`, the matrix is
/// `cos(theta / 2) I - i sin(theta / 2) Y`.
pub fn eval_ry_big(params: &[Param]) -> BigEndian1Q {
    let theta = scalar_param(params, 0.0);
    let (s, c) = (theta / 2.0).sin_cos();
    let s_real = Complex::new(s, 0.0);
    let neg_s_real = Complex::new(-s, 0.0);
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        Complex::new(c, 0.0), neg_s_real,
        s_real, Complex::new(c, 0.0),
    ])
}

/// Builds a physical controlled-`Rz(theta)` gate in big-endian ordering.
///
/// The control qubit is the most significant qubit. In the two-qubit
/// big-endian basis `|00>, |01>, |10>, |11>`, this is the block-diagonal matrix
/// `diag(I, Rz(theta))`.
pub fn eval_crz_big(params: &[Param]) -> BigEndian2Q {
    let target_gate = eval_rz_big(params);
    build_controlled_gate_big_endian(&target_gate)
}

/// Builds the Cirq-style `CXPow(t)` matrix in big-endian ordering.
///
/// The parameter is an exponent `t`, not a physical angle. The controlled block
/// is `XPow(t) = exp(i pi t / 2) Rx(pi t)`, so `CXPow(1)` is exactly the full
/// controlled-X gate in the `|00>, |01>, |10>, |11>` basis.
pub fn eval_cx_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = scalar_param(params, 1.0);
    let x_pow = eval_x_pow_big(exponent);
    build_controlled_gate_big_endian(&x_pow)
}

/// Builds the Cirq-style `CYPow(t)` matrix in big-endian ordering.
///
/// The parameter is an exponent `t`, not a physical angle. The controlled block
/// is `YPow(t) = exp(i pi t / 2) Ry(pi t)`, so `CYPow(1)` is exactly the full
/// controlled-Y gate.
pub fn eval_cy_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = scalar_param(params, 1.0);
    let y_pow = eval_y_pow_big(exponent);
    build_controlled_gate_big_endian(&y_pow)
}

/// Builds the Cirq-style `CZPow(t)` matrix in big-endian ordering.
///
/// The parameter is an exponent `t`, not a physical angle. The resulting matrix
/// is `diag(1, 1, 1, exp(i pi t))`, so `CZPow(1)` is exactly CZ.
pub fn eval_cz_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = scalar_param(params, 1.0);
    let z_pow = eval_z_pow_big(exponent);
    build_controlled_gate_big_endian(&z_pow)
}
