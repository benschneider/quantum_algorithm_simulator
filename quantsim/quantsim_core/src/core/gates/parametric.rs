use crate::core::endian_utils::build_controlled_gate_big_endian;
use crate::core::types::{BigEndian1Q, BigEndian2Q, Param};
use nalgebra::{Complex, OMatrix, U2};
use std::f32::consts::PI;

/// Builds the Rz rotation gate matrix in big-endian ordering.
pub fn eval_rz_big(params: &[Param]) -> BigEndian1Q {
    let theta = if let Some(Param::Scalar(theta)) = params.first() {
        *theta
    } else {
        0.0
    };
    let e_itheta2 = (Complex::new(0.0, -theta / 2.0)).exp();
    let e_i_theta2_conj = e_itheta2.conj();
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        e_i_theta2_conj, Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0), e_itheta2,
    ])
}

/// Builds the Rx rotation gate matrix in big-endian ordering.
pub fn eval_rx_big(params: &[Param]) -> BigEndian1Q {
    let theta = if let Some(Param::Scalar(theta)) = params.first() {
        *theta
    } else {
        0.0
    };
    let (s, c) = (theta / 2.0).sin_cos();
    let i_s = Complex::new(0.0, -s);
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        Complex::new(c, 0.0), i_s,
        i_s, Complex::new(c, 0.0),
    ])
}

/// Builds the Ry rotation gate matrix in big-endian ordering.
pub fn eval_ry_big(params: &[Param]) -> BigEndian1Q {
    let theta = if let Some(Param::Scalar(theta)) = params.first() {
        *theta
    } else {
        0.0
    };
    let (s, c) = (theta / 2.0).sin_cos();
    let s_real = Complex::new(s, 0.0);
    let neg_s_real = Complex::new(-s, 0.0);
    OMatrix::<Complex<f32>, U2, U2>::from_row_slice(&[
        Complex::new(c, 0.0), neg_s_real,
        s_real, Complex::new(c, 0.0),
    ])
}

/// Builds the controlled-Rz rotation gate matrix in big-endian ordering.
pub fn eval_crz_big(params: &[Param]) -> BigEndian2Q {
    let target_gate = eval_rz_big(params);
    build_controlled_gate_big_endian(&target_gate)
}

/// Builds the controlled-X^pow gate matrix in big-endian ordering.
pub fn eval_cx_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = if let Some(Param::Scalar(exp)) = params.first() {
        *exp
    } else {
        1.0
    };
    let x_pow = eval_rx_big(&[Param::Scalar(exponent * PI / 2.0)]);
    build_controlled_gate_big_endian(&x_pow)
}

/// Builds the controlled-Y^pow gate matrix in big-endian ordering.
pub fn eval_cy_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = if let Some(Param::Scalar(exp)) = params.first() {
        *exp
    } else {
        1.0
    };
    let y_pow = eval_ry_big(&[Param::Scalar(exponent * PI / 2.0)]);
    build_controlled_gate_big_endian(&y_pow)
}

/// Builds the controlled-Z^pow gate matrix in big-endian ordering.
pub fn eval_cz_pow_big(params: &[Param]) -> BigEndian2Q {
    let exponent = if let Some(Param::Scalar(exp)) = params.first() {
        *exp
    } else {
        1.0
    };
    let z_pow = eval_rz_big(&[Param::Scalar(exponent * PI)]);
    build_controlled_gate_big_endian(&z_pow)
}