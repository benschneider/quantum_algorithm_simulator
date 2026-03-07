//use crate::core::endian_utils::to_sparse_little_endian;
use crate::core::gates::parametric::{eval_rx_big, eval_ry_big, eval_rz_big};
use crate::core::types::{Arity, Gate, GateMatrix, Param};
use dyn_clone::DynClone;
use nalgebra::{Complex, DMatrix};
//use nalgebra_sparse::{CooMatrix, CsrMatrix};
use std::collections::HashMap;
use std::f32::consts::PI;
use strum::IntoEnumIterator;

#[inline]
fn c(re: f32, im: f32) -> Complex<f32> {
    Complex::new(re, im)
}

fn dense_from_small_matrix<const R: usize, const C: usize, S>(
    matrix: &nalgebra::Matrix<Complex<f32>, nalgebra::Const<R>, nalgebra::Const<C>, S>,
) -> DMatrix<Complex<f32>>
where
    S: nalgebra::storage::Storage<Complex<f32>, nalgebra::Const<R>, nalgebra::Const<C>>,
{
    DMatrix::from_fn(R, C, |r, c| matrix[(r, c)])
}

/// Trait for gate evaluation functions.
pub trait EvalFn: DynClone + Send + Sync {
    fn call(&self, params: &[Param], qubits: &[u32]) -> GateMatrix;
}

impl<F> EvalFn for F
where
    F: for<'a> Fn(&'a [Param], &[u32]) -> GateMatrix + DynClone + Send + Sync + 'static,
{
    fn call(&self, params: &[Param], qubits: &[u32]) -> GateMatrix {
        (self)(params, qubits)
    }
}

/// Updated EigenGate trait to return big-endian DMatrix.
pub trait EigenGate: DynClone + Send + Sync {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>>;
}

dyn_clone::clone_trait_object!(EigenGate);

/// Cirq-style `XPow(t) = exp(i pi t / 2) Rx(pi t)` in big-endian basis order.
#[derive(Clone)]
pub struct XPowGate;
impl EigenGate for XPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        let global_phase = (c(0.0, PI * exponent / 2.0)).exp();
        let angle = exponent * PI / 2.0;
        let (s, c_val) = angle.sin_cos();
        let i_s = c(0.0, -s);
        let c_real = c(c_val, 0.0);
        let mut matrix = DMatrix::zeros(2, 2);
        matrix[(0, 0)] = c_real * global_phase;
        matrix[(0, 1)] = i_s * global_phase;
        matrix[(1, 0)] = i_s * global_phase;
        matrix[(1, 1)] = c_real * global_phase;
        matrix
    }
}

/// Cirq-style `YPow(t) = exp(i pi t / 2) Ry(pi t)` in big-endian basis order.
#[derive(Clone)]
pub struct YPowGate;
impl EigenGate for YPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        let global_phase = (c(0.0, PI * exponent / 2.0)).exp();
        let angle = exponent * PI / 2.0;
        let (s, c_val) = angle.sin_cos();
        let c_real = c(c_val, 0.0);
        let s_real = c(s, 0.0);
        let neg_s_real = c(-s, 0.0);
        let mut matrix = DMatrix::zeros(2, 2);
        matrix[(0, 0)] = c_real * global_phase;
        matrix[(0, 1)] = neg_s_real * global_phase;
        matrix[(1, 0)] = s_real * global_phase;
        matrix[(1, 1)] = c_real * global_phase;
        matrix
    }
}

/// Cirq-style `ZPow(t) = diag(1, exp(i pi t))` in big-endian basis order.
#[derive(Clone)]
pub struct ZPowGate;
impl EigenGate for ZPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        let angle = exponent * PI;
        let e_i_pi_t = c(angle.cos(), angle.sin());
        let mut matrix = DMatrix::zeros(2, 2);
        matrix[(0, 0)] = c(1.0, 0.0);
        matrix[(1, 1)] = e_i_pi_t;
        matrix
    }
}

/// Controlled Cirq-style `CCZPow(t)` with phase applied only to `|111>`.
#[derive(Clone)]
pub struct CCZPowGate;
impl EigenGate for CCZPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        let angle = exponent * PI;
        let mut matrix = DMatrix::identity(8, 8);
        matrix[(7, 7)] = c(angle.cos(), angle.sin());
        matrix
    }
}

/// Controlled Cirq-style `CXPow(t)` with control on the most significant qubit.
#[derive(Clone)]
pub struct CXPowGate;
impl EigenGate for CXPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        // Create a controlled version of XPow
        let xpow_gate = XPowGate;
        let target_matrix = xpow_gate.matrix_at_big(exponent);
        Self::create_controlled_matrix(target_matrix)
    }
}

impl CXPowGate {
    fn create_controlled_matrix(target_matrix: DMatrix<Complex<f32>>) -> DMatrix<Complex<f32>> {
        let n = target_matrix.nrows();
        let mut controlled_matrix = DMatrix::identity(2 * n, 2 * n);
        controlled_matrix.view_mut((n, n), (n, n)).copy_from(&target_matrix);
        controlled_matrix
    }
}

/// Controlled Cirq-style `CYPow(t)` with control on the most significant qubit.
#[derive(Clone)]
pub struct CYPowGate;
impl EigenGate for CYPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        // Create a controlled version of YPow
        let ypow_gate = YPowGate;
        let target_matrix = ypow_gate.matrix_at_big(exponent);
        Self::create_controlled_matrix(target_matrix)
    }
}

impl CYPowGate {
    fn create_controlled_matrix(target_matrix: DMatrix<Complex<f32>>) -> DMatrix<Complex<f32>> {
        let n = target_matrix.nrows();
        let mut controlled_matrix = DMatrix::identity(2 * n, 2 * n);
        controlled_matrix.view_mut((n, n), (n, n)).copy_from(&target_matrix);
        controlled_matrix
    }
}

/// Controlled Cirq-style `CZPow(t)` with phase on `|11>`.
#[derive(Clone)]
pub struct CZPowGate;
impl EigenGate for CZPowGate {
    fn matrix_at_big(&self, exponent: f32) -> DMatrix<Complex<f32>> {
        // Create a controlled version of ZPow
        let zpow_gate = ZPowGate;
        let target_matrix = zpow_gate.matrix_at_big(exponent);
        Self::create_controlled_matrix(target_matrix)
    }
}

impl CZPowGate {
    fn create_controlled_matrix(target_matrix: DMatrix<Complex<f32>>) -> DMatrix<Complex<f32>> {
        let n = target_matrix.nrows();
        let mut controlled_matrix = DMatrix::identity(2 * n, 2 * n);
        controlled_matrix.view_mut((n, n), (n, n)).copy_from(&target_matrix);
        controlled_matrix
    }
}

pub enum GateKind {
    Unitary {
        eval: Box<dyn EvalFn>,
        is_parametric: bool,
    },
    Eigen {
        gate: Box<dyn EigenGate>,
    },
    Custom {
        matrix: DMatrix<Complex<f32>>,
    },
}

impl Clone for GateKind {
    fn clone(&self) -> Self {
        match self {
            Self::Unitary {
                eval,
                is_parametric,
            } => Self::Unitary {
                eval: dyn_clone::clone_box(eval.as_ref()),
                is_parametric: *is_parametric,
            },
            Self::Eigen { gate } => Self::Eigen {
                gate: dyn_clone::clone_box(gate.as_ref()),
            },
            Self::Custom { matrix } => Self::Custom {
                matrix: matrix.clone(),
            },
        }
    }
}


/// Metadata for a quantum gate.
#[derive(Debug, Clone)]
pub struct GateMeta {
    pub id: Gate,
    pub label: String,
    pub description: String,
    pub arity: Arity,
    pub is_parametric: bool,
    pub qubits: Option<Vec<usize>>,
}

/// A registry for storing and managing quantum gate definitions.
#[derive(Clone)]
pub struct GateRegistry {
    gates: HashMap<Gate, (GateMeta, GateKind)>,
}

impl std::fmt::Debug for GateRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let gate_ids: Vec<_> = self.gates.keys().collect();
        f.debug_struct("GateRegistry")
            .field("gates", &gate_ids)
            .finish()
    }
}

impl GateRegistry {
    fn new() -> Self {
        Self {
            gates: HashMap::new(),
        }
    }

    pub fn new_with_standard_gates() -> Self {
        let mut registry = Self::new();

        for gate in Gate::iter() {
            let (meta, kind) = registry.gate_info(gate.clone());
            registry.gates.insert(gate, (meta, kind));
        }

        registry
    }


    fn create_one_qubit_unitary(matrix_data: &[Complex<f32>]) -> GateKind {
        let matrix_data = matrix_data.to_vec();
        GateKind::Unitary {
            eval: Box::new(move |_: &[_], _: &[u32]| {
                let big_matrix = DMatrix::from_row_slice(2, 2, &matrix_data);
                GateMatrix::BigEndian(big_matrix)
            }),
            is_parametric: false,
        }
    }

    fn create_controlled_gate(target_gate: DMatrix<Complex<f32>>) -> GateKind {
        let n = target_gate.nrows();
        let mut controlled_matrix = DMatrix::identity(2 * n, 2 * n);
        controlled_matrix.view_mut((n, n), (n, n)).copy_from(&target_gate);

        GateKind::Unitary {
            eval: Box::new(move |_: &[_], _: &[u32]| {
                GateMatrix::BigEndian(controlled_matrix.clone())
            }),
            is_parametric: false,
        }
    }


    fn gate_info(&self, gate: Gate) -> (GateMeta, GateKind) {
        let sqrt_2_inv = std::f32::consts::FRAC_1_SQRT_2;

        let (label, description, arity, is_parametric, qubits, kind) = match gate {
            Gate::H => (
                "H",
                "Hadamard gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // H = [[1, 1], [1, -1]] / sqrt(2)
                Self::create_one_qubit_unitary(&[
                    c(sqrt_2_inv, 0.0), c(sqrt_2_inv, 0.0),
                    c(sqrt_2_inv, 0.0), c(-sqrt_2_inv, 0.0),
                ]),
            ),
            Gate::X => (
                "X",
                "Pauli-X gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // X = [[0, 1], [1, 0]]
                Self::create_one_qubit_unitary(&[
                    c(0.0, 0.0), c(1.0, 0.0),
                    c(1.0, 0.0), c(0.0, 0.0),
                ]),
            ),
            Gate::Y => (
                "Y",
                "Pauli-Y gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // Y = [[0, -i], [i, 0]]
                Self::create_one_qubit_unitary(&[
                    c(0.0, 0.0), c(0.0, -1.0),
                    c(0.0, 1.0), c(0.0, 0.0),
                ]),
            ),
            Gate::Z => (
                "Z",
                "Pauli-Z gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // Z = [[1, 0], [0, -1]]
                Self::create_one_qubit_unitary(&[
                    c(1.0, 0.0), c(0.0, 0.0),
                    c(0.0, 0.0), c(-1.0, 0.0),
                ]),
            ),
            Gate::SqrtX => (
                "√X",
                "Square root of X gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // √X = [[0.5+0.5i, 0.5-0.5i], [0.5-0.5i, 0.5+0.5i]]
                Self::create_one_qubit_unitary(&[
                    c(0.5, 0.5), c(0.5, -0.5),
                    c(0.5, -0.5), c(0.5, 0.5),
                ]),
            ),
            Gate::SqrtY => (
                "√Y",
                "Square root of Y gate",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // √Y = [[0.5+0.5i, -0.5-0.5i], [0.5+0.5i, 0.5+0.5i]]
                Self::create_one_qubit_unitary(&[
                    c(0.5, 0.5), c(-0.5, -0.5),
                    c(0.5, 0.5), c(0.5, 0.5),
                ]),
            ),
            Gate::SqrtZ => (
                "√Z",
                "Square root of Z gate (S gate)",
                Arity::OneQ,
                false,
                Some(vec![0]),
                // S = [[1, 0], [0, i]]
                Self::create_one_qubit_unitary(&[
                    c(1.0, 0.0), c(0.0, 0.0),
                    c(0.0, 0.0), c(0.0, 1.0),
                ]),
            ),
            Gate::Rx => (
                "Rx",
                "Physical X-axis rotation by an angle in radians",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Unitary {
                    eval: Box::new(|params: &[_], _: &[u32]| {
                        GateMatrix::BigEndian(dense_from_small_matrix(&eval_rx_big(params)))
                    }),
                    is_parametric: true,
                },
            ),
            Gate::Ry => (
                "Ry",
                "Physical Y-axis rotation by an angle in radians",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Unitary {
                    eval: Box::new(|params: &[_], _: &[u32]| {
                        GateMatrix::BigEndian(dense_from_small_matrix(&eval_ry_big(params)))
                    }),
                    is_parametric: true,
                },
            ),
            Gate::Rz => (
                "Rz",
                "Physical Z-axis rotation by an angle in radians",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Unitary {
                    eval: Box::new(|params: &[_], _: &[u32]| {
                        GateMatrix::BigEndian(dense_from_small_matrix(&eval_rz_big(params)))
                    }),
                    is_parametric: true,
                },
            ),
            Gate::CZ => (
                "CZ",
                "Controlled-Z gate",
                Arity::TwoQ,
                false,
                Some(vec![0, 1]),
                Self::create_controlled_gate(DMatrix::from_row_slice(
                    2, 2,
                    &[ // Z matrix
                        c(1.0, 0.0), c(0.0, 0.0),
                        c(0.0, 0.0), c(-1.0, 0.0),
                    ],
                )),
            ),
            Gate::CCZ => (
                "CCZ",
                "Toffoli gate (CCZ)",
                Arity::ThreeQ,
                false,
                Some(vec![0, 1, 2]),
                GateKind::Unitary {
                    eval: Box::new(|_: &[_], _: &[u32]| {
                        let mut big_matrix = DMatrix::identity(8, 8);
                        big_matrix[(7, 7)] = c(-1.0, 0.0);
                        GateMatrix::BigEndian(big_matrix)
                    }),
                    is_parametric: false,
                },
            ),
            Gate::SWAP => (
                "SWAP",
                "SWAP gate",
                Arity::TwoQ,
                false,
                Some(vec![0, 1]),
                GateKind::Unitary {
                    eval: Box::new(|_: &[_], _: &[u32]| {
                        let mut big_matrix = DMatrix::zeros(4, 4);
                        big_matrix[(0, 0)] = c(1.0, 0.0);
                        big_matrix[(1, 2)] = c(1.0, 0.0);
                        big_matrix[(2, 1)] = c(1.0, 0.0);
                        big_matrix[(3, 3)] = c(1.0, 0.0);
                        GateMatrix::BigEndian(big_matrix)
                    }),
                    is_parametric: false,
                },
            ),
            Gate::MOVE => (
                "MOVE",
                "Logical state-transfer gate; exchanges |01> and |10> without implying a physical calibrated pulse",
                Arity::TwoQ,
                false,
                Some(vec![0, 1]),
                GateKind::Unitary {
                    eval: Box::new(|_: &[_], _: &[u32]| {
                        let mut big_matrix = DMatrix::zeros(4, 4);
                        big_matrix[(0, 0)] = c(1.0, 0.0);
                        big_matrix[(1, 2)] = c(1.0, 0.0);
                        big_matrix[(2, 1)] = c(1.0, 0.0);
                        big_matrix[(3, 3)] = c(1.0, 0.0);
                        GateMatrix::BigEndian(big_matrix)
                    }),
                    is_parametric: false,
                },
            ),
            Gate::CCZPow => (
                "CCZPow",
                "Controlled-controlled-Z phase gate",
                Arity::ThreeQ,
                true,
                Some(vec![0, 1, 2]),
                GateKind::Eigen {
                    gate: Box::new(CCZPowGate),
                },
            ),
            Gate::CX => (
                "CX",
                "Controlled-X (CNOT) gate",
                Arity::TwoQ,
                false,
                Some(vec![0, 1]),
                GateKind::Unitary {
                    eval: Box::new(|_params: &[_], qubits: &[u32]| {
                        let big_matrix = if qubits.len() == 2 && qubits[0] > qubits[1] {
                            // Reversed CNOT (control on LSB). Flips MSB if LSB is 1.
                            // Swaps |01> and |11>.
                            // [[1, 0, 0, 0],  // |00> -> |00>
                            //  [0, 0, 0, 1],  // |01> -> |11>
                            //  [0, 0, 1, 0],  // |10> -> |10>
                            //  [0, 1, 0, 0]]  // |11> -> |01>
                            DMatrix::from_row_slice(4, 4, &[
                                c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0),
                                c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0),
                                c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0),
                                c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0),
                            ]).transpose()
                        } else {
                            // Standard CNOT (control on MSB). Flips LSB if MSB is 1.
                            // Swaps |10> and |11>.
                            //[[1, 0, 0, 0],  // |00> -> |00>
                            // [0, 1, 0, 0],  // |01> -> |01>
                            // [0, 0, 0, 1],  // |10> -> |11>
                            // [0, 0, 1, 0]]  // |11> -> |10>
                            DMatrix::from_row_slice(4, 4, &[
                                c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0),
                                c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0),
                                c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0),
                                c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0), c(0.0, 0.0),
                            ]).transpose()
                        };
                        GateMatrix::BigEndian(big_matrix)
                    }),
                    is_parametric: false,
                },
            ),
            Gate::CY => (
                "CY",
                "Controlled-Y gate",
                Arity::TwoQ,
                false,
                Some(vec![0, 1]),
                Self::create_controlled_gate(DMatrix::from_row_slice(
                    2, 2,
                    &[ // Y matrix
                        c(0.0, 0.0), c(0.0, -1.0),
                        c(0.0, 1.0), c(0.0, 0.0),
                    ],
                )),
            ),
            Gate::CCNOT => (
                "CCNOT",
                "Toffoli (CCNOT) gate",
                Arity::ThreeQ,
                false,
                Some(vec![0, 1, 2]),
                GateKind::Unitary {
                    eval: Box::new(|_params: &[_], qubits: &[u32]| {
                        let mut big_matrix = DMatrix::identity(8, 8);
                        let control1 = qubits[0];
                        let control2 = qubits[1];
                        let target = qubits[2];

                        for i in 0..8 {
                            // Check if control bits are set for this basis state.
                            if (i >> control1) & 1 == 1 && (i >> control2) & 1 == 1 {
                                // Find the state to swap with by flipping the target bit.
                                let j = i ^ (1 << target);

                                // To avoid performing the swap twice, only modify the matrix
                                // for the smaller of the two indices in the pair.
                                if i < j {
                                    // Zero out the diagonal elements for the pair.
                                    big_matrix[(i, i)] = c(0.0, 0.0);
                                    big_matrix[(j, j)] = c(0.0, 0.0);

                                    // Set the off-diagonal elements to perform the swap.
                                    big_matrix[(i, j)] = c(1.0, 0.0);
                                    big_matrix[(j, i)] = c(1.0, 0.0);
                                }
                            }
                        }
                        GateMatrix::BigEndian(big_matrix)
                    }),
                    is_parametric: false,
                },
            ),
            Gate::CRz => (
                "CRz",
                "Controlled phase gate diag(1, 1, 1, exp(i theta)) in big-endian order",
                Arity::TwoQ,
                true,
                Some(vec![0, 1]),
                GateKind::Unitary {
                    eval: Box::new(|params: &[_], _: &[u32]| {
                        if let Some(Param::Scalar(theta)) = params.first() {
                            let angle = *theta;
                            let e_i_theta = c(angle.cos(), angle.sin());
                            let mut matrix = DMatrix::identity(4, 4);
                            matrix[(3, 3)] = e_i_theta;
                            GateMatrix::BigEndian(matrix)
                        } else {
                            // If no parameter is provided, return a BigEndian identity matrix.
                            GateMatrix::BigEndian(DMatrix::identity(4, 4))
                        }
                    }),
                    is_parametric: true,
                },
            ),
            Gate::CXPow => (
                "CXPow",
                "Cirq-style controlled XPow exponent gate; exponent 1 is full CX",
                Arity::TwoQ,
                true,
                Some(vec![0, 1]),
                GateKind::Eigen {
                    gate: Box::new(CXPowGate),
                },
            ),
            Gate::CYPow => (
                "CYPow",
                "Cirq-style controlled YPow exponent gate; exponent 1 is full CY",
                Arity::TwoQ,
                true,
                Some(vec![0, 1]),
                GateKind::Eigen {
                    gate: Box::new(CYPowGate),
                },
            ),
            Gate::CZPow => (
                "CZPow",
                "Cirq-style controlled ZPow exponent gate; exponent 1 is full CZ",
                Arity::TwoQ,
                true,
                Some(vec![0, 1]),
                GateKind::Eigen {
                    gate: Box::new(CZPowGate),
                },
            ),
            Gate::XPow => (
                "XPow",
                "Cirq-style XPow exponent gate; XPow(t) = exp(i pi t / 2) Rx(pi t)",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Eigen {
                    gate: Box::new(XPowGate),
                },
            ),
            Gate::YPow => (
                "YPow",
                "Cirq-style YPow exponent gate; YPow(t) = exp(i pi t / 2) Ry(pi t)",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Eigen {
                    gate: Box::new(YPowGate),
                },
            ),
            Gate::ZPow => (
                "ZPow",
                "Cirq-style ZPow exponent gate; ZPow(t) = diag(1, exp(i pi t))",
                Arity::OneQ,
                true,
                Some(vec![0]),
                GateKind::Eigen {
                    gate: Box::new(ZPowGate),
                },
            ),
            Gate::Custom => (
                "Custom",
                "Custom gate",
                Arity::OneQ, // This is a placeholder, arity is determined by the matrix
                false,
                None,
                GateKind::Custom {
                    matrix: DMatrix::identity(2, 2),
                },
            ),
        };

        (
            GateMeta {
                id: gate,
                label: label.to_string(),
                description: description.to_string(),
                arity,
                is_parametric,
                qubits,
            },
            kind,
        )
    }

    pub fn get_meta(&self, id: &Gate) -> Option<&GateMeta> {
        self.gates.get(id).map(|(meta, _)| meta)
    }

    pub fn eval(&self, operation_id: &Gate, params: &[Param], qubits: &[u32]) -> Option<GateMatrix> {
        self.gates.get(operation_id).map(|(_, kind)| match kind {
            GateKind::Unitary { eval, .. } => eval.call(params, qubits),
            GateKind::Eigen { gate } => {
                let exponent = params
                    .first()
                    .and_then(|p| match p {
                        Param::Scalar(val) => Some(*val),
                    })
                    .unwrap_or(0.0);
                let big_matrix = gate.matrix_at_big(exponent);
                GateMatrix::BigEndian(big_matrix)
            }
            GateKind::Custom { matrix } => {
                // This branch is a placeholder. Real custom gates are handled by
                // `prepare_for_execution` using the matrix on the `Operation`.
                GateMatrix::BigEndian(matrix.clone())
            }
        })
    }


    pub fn ids(&self) -> impl Iterator<Item = &Gate> {
        self.gates.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Gate, &GateMeta)> {
        self.gates.iter().map(|(id, (meta, _))| (id, meta))
    }
}

impl Default for GateRegistry {
    fn default() -> Self {
        Self::new_with_standard_gates()
    }
}
