//! # Types
//!
//! This module defines common data structures and type aliases used throughout
//! the `quantsim` library. These types provide the foundation for representing
//! - quantum circuits, gates, and simulation states.
use nalgebra::{Complex, DMatrix, OMatrix, U2, U4, U8};
use nalgebra_sparse::csr::CsrMatrix;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use smallvec::SmallVec;
use strum::EnumIter;

/// Reads both the row arrays used in circuit templates and nalgebra's saved matrix format.
fn deserialize_complex_matrix<'de, D>(deserializer: D) -> Result<DMatrix<Complex<f32>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Coefficient {
        Real(f32),
        Complex(Complex<f32>),
    }

    #[derive(Deserialize)]
    #[serde(untagged, expecting = "matrix rows or [column-major coefficients, rows, columns]")]
    enum MatrixData {
        ColumnMajor(Vec<Complex<f32>>, usize, usize),
        Rows(Vec<Vec<Coefficient>>),
    }

    match MatrixData::deserialize(deserializer)? {
        MatrixData::ColumnMajor(values, rows, cols) => {
            if rows.checked_mul(cols) != Some(values.len()) {
                return Err(D::Error::custom(
                    "matrix dimensions do not match coefficient count",
                ));
            }
            Ok(DMatrix::from_vec(rows, cols, values))
        }
        MatrixData::Rows(rows) => {
            let nrows = rows.len();
            let ncols = rows.first().map_or(0, Vec::len);
            if rows.iter().any(|row| row.len() != ncols) {
                return Err(D::Error::custom(
                    "all rows must have the same number of columns",
                ));
            }
            if ncols == 0 {
                return Ok(DMatrix::zeros(0, 0));
            }
            let values = rows.into_iter().flatten().map(|value| match value {
                Coefficient::Real(re) => Complex::new(re, 0.0),
                Coefficient::Complex(c) => c,
            });
            Ok(DMatrix::from_row_iterator(nrows, ncols, values))
        }
    }
}

/// A 2x2 complex matrix, typically used for single-qubit gates.
pub type CMat2 = [[Complex<f32>; 2]; 2];
/// A 4x4 complex matrix, typically used for two-qubit gates.
pub type CMat4 = [[Complex<f32>; 4]; 4];
/// A 8x8 complex matrix, typically used for three-qubit gates.
pub type CMat8 = [[Complex<f32>; 8]; 8];

/// Type aliases for big-endian dense matrices used in gate construction.
pub type BigEndian1Q = OMatrix<Complex<f32>, U2, U2>;
pub type BigEndian2Q = OMatrix<Complex<f32>, U4, U4>;
pub type BigEndian3Q = OMatrix<Complex<f32>, U8, U8>;

/// Represents the number of qubits a gate operates on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Arity {
    /// A single-qubit gate.
    OneQ,
    /// A two-qubit gate.
    TwoQ,
    /// A three-qubit gate.
    ThreeQ,
}

impl Arity {
    /// Returns the arity as a `usize`.
    pub fn as_usize(&self) -> usize {
        match self {
            Arity::OneQ => 1,
            Arity::TwoQ => 2,
            Arity::ThreeQ => 3,
        }
    }
}

/// Represents a parameter for a quantum gate, typically an angle.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Param {
    /// A scalar parameter, represented as a 32-bit float.
    Scalar(f32),
}

/// Represents the matrix of a quantum gate.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum GateMatrix {
    /// A dense, big-endian matrix, suitable for user-facing definitions.
    BigEndian(#[serde(deserialize_with = "deserialize_complex_matrix")] DMatrix<Complex<f32>>),
    /// A sparse, little-endian matrix, required for the simulation engine.
    LittleEndian(CsrMatrix<Complex<f32>>),
}

impl GateMatrix {
    /// Converts the gate matrix to a dense `DMatrix`.
    pub fn to_dmatrix(&self) -> DMatrix<Complex<f32>> {
        match self {
            GateMatrix::LittleEndian(m) => {
                let mut dense_matrix = DMatrix::zeros(m.nrows(), m.ncols());
                for (r, c, val) in m.triplet_iter() {
                    dense_matrix[(r, c)] = *val;
                }
                dense_matrix
            }
            GateMatrix::BigEndian(m) => m.clone(),
        }
    }

    /// Converts the gate matrix to a sparse `CsrMatrix`.
    pub fn to_sparse_matrix(&self) -> CsrMatrix<Complex<f32>> {
        match self {
            GateMatrix::LittleEndian(m) => m.clone(),
            GateMatrix::BigEndian(m) => {
                let mut coo = nalgebra_sparse::CooMatrix::new(m.nrows(), m.ncols());
                for r in 0..m.nrows() {
                    for c in 0..m.ncols() {
                        let val = m[(r, c)];
                        if val.norm_sqr() > 1e-9 {
                            coo.push(r, c, val);
                        }
                    }
                }
                CsrMatrix::from(&coo)
            }
        }
    }
}

impl std::fmt::Display for GateMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateMatrix::LittleEndian(m) => {
                write!(
                    f,
                    "LittleEndian Sparse Matrix ({}x{}) with {} non-zero elements",
                    m.nrows(),
                    m.ncols(),
                    m.nnz()
                )
            }
            GateMatrix::BigEndian(m) => {
                write!(f, "BigEndian Dense Matrix ({}x{})", m.nrows(), m.ncols())
            }
        }
    }
}

impl From<GateMatrix> for DMatrix<Complex<f32>> {
    fn from(value: GateMatrix) -> Self {
        value.to_dmatrix()
    }
}

/// Represents a single operation in a quantum circuit.
///
/// An operation consists of a quantum gate applied to specific qubits with
/// optional parameters. Operations within the same circuit timestep must
/// act on disjoint sets of qubits.
///
/// # Examples
///
/// Create a Hadamard gate operation:
/// ```rust
/// use quantsim_core::core::types::{Operation, Gate};
///
/// let h_gate = Operation::new(Gate::H, vec![0], vec![]);
/// ```
///
/// Create a rotation gate with parameters:
/// ```rust
/// let rx_gate = Operation::new(Gate::Rx, vec![1], vec![std::f32::consts::PI / 2.0]);
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Operation {
    /// The unique identifier for the gate.
    pub id: Gate,
    /// An optional label for this operation, such as its role in a circuit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A list of parameters for the gate.
    #[serde(default)]
    pub params: SmallVec<[Param; 1]>,
    /// The qubits the gate acts on.
    pub qubits: SmallVec<[u32; 2]>,
    /// The matrix representation of the gate, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<GateMatrix>,
}

impl Operation {
    /// Creates a new `Operation`.
    pub fn new(id: Gate, qubits: Vec<u32>, params: Vec<f32>) -> Self {
        Self {
            id,
            name: None,
            qubits: qubits.into_iter().collect(),
            params: params.into_iter().map(Param::Scalar).collect(),
            matrix: None,
        }
    }

    /// Creates a new `Operation` for editing purposes.
    pub fn new_for_editing(id: Gate, params: Vec<f32>, arity: Arity) -> Self {
        Self {
            id,
            name: None,
            qubits: (0u32..arity.as_usize() as u32).collect(),
            params: params.into_iter().map(Param::Scalar).collect(),
            matrix: None,
        }
    }
}

/// Represents the different types of quantum gates supported by the simulator.
///
/// This enum includes standard quantum gates like Pauli gates, controlled gates,
/// rotation gates, and custom gates. Each variant corresponds to a specific
/// unitary operation that can be applied to qubits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum Gate {
    H, X, Y, Z,
    SqrtX, SqrtY, SqrtZ,
    CX, CZ, CY, SWAP, MOVE,
    CCNOT, CCZ,
    Rz, Rx, Ry,
    CRz,
    CXPow, CYPow, CZPow,
    XPow, YPow, ZPow,
    CCZPow,
    Custom,
}

impl std::fmt::Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents the initial state preparation for a qubit.
///
/// When building a circuit, you can specify how each qubit should be initialized
/// before applying the circuit operations. This allows starting from states
/// other than the default |0⟩ state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QubitPrep {
    /// The |0⟩ state.
    Zero,
    /// The |1⟩ state.
    One,
    /// The |+⟩ state.
    Plus,
    /// The |-⟩ state.
    Minus,
}

#[cfg(test)]
mod matrix_json_tests {
    use super::*;
    use crate::core::circuit::CircuitData;

    #[test]
    fn grover_custom_gates_survive_a_json_round_trip() {
        let grover = crate::circuits::load_template("grover_algorithm").unwrap();
        let data = CircuitData::from(grover);
        let json = serde_json::to_string_pretty(&data).unwrap();
        let reloaded: CircuitData = serde_json::from_str(&json).unwrap();
        assert_eq!(reloaded.steps, data.steps);
    }

    #[test]
    fn matrix_rows_and_saved_column_major_coefficients_agree() {
        let rows = r#"{"BigEndian":[[1,[2,-1]],[[3,4],0]]}"#;
        let saved = r#"{"BigEndian":[[[1,0],[3,4],[2,-1],[0,0]],2,2]}"#;
        let matrix: GateMatrix = serde_json::from_str(rows).unwrap();
        assert_eq!(matrix.to_dmatrix()[(0, 1)], Complex::new(2.0, -1.0));
        assert_eq!(serde_json::from_str::<GateMatrix>(saved).unwrap(), matrix);
    }

    #[test]
    fn invalid_matrix_dimensions_return_errors() {
        for json in [
            r#"{"BigEndian":[[1,0],[1]]}"#,
            r#"{"BigEndian":[[[1,0]],2,2]}"#,
            r#"{"BigEndian":[[],18446744073709551615,2]}"#,
        ] {
            assert!(serde_json::from_str::<GateMatrix>(json).is_err());
        }
    }
}
