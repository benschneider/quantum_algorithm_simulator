//! Apply one authored column of disjoint gates on a two-row ladder.
use crate::ColumnMps;
use nalgebra::DMatrix;
use num_complex::Complex64;
use quantsim_core::types::Operation;

enum Position {
    Single { column: usize, row: usize },
    Rung(usize),
    Leg { column: usize, row: usize },
}

fn position(op: &Operation, columns: usize) -> Result<Position, &'static str> {
    match *op.qubits.as_slice() {
        [a] => Ok(Position::Single {
            column: a as usize % columns,
            row: a as usize / columns,
        }),
        [a, b] if a != b && a as usize % columns == b as usize % columns => {
            Ok(Position::Rung(a as usize % columns))
        }
        [a, b] if a.abs_diff(b) == 1 && a as usize / columns == b as usize / columns => {
            Ok(Position::Leg {
                column: a.min(b) as usize % columns,
                row: a as usize / columns,
            })
        }
        _ => Err("gate does not address a single wire or an edge of the ladder"),
    }
}

fn ordered_matrix(
    op: &Operation,
    matrix: &mut impl FnMut(&Operation) -> DMatrix<Complex64>,
) -> Result<DMatrix<Complex64>, &'static str> {
    let mut gate = matrix(op);
    let dim = 1 << op.qubits.len();
    if gate.shape() != (dim, dim) {
        return Err("gate matrix size does not match its ladder operands");
    }
    // The callback uses authored |a,b⟩ order. ColumnMps uses top-before-bottom
    // for rungs and left-before-right for legs, both ascending wire numbers.
    if matches!(*op.qubits.as_slice(), [a, b] if a > b) {
        gate.swap_rows(1, 2);
        gate.swap_columns(1, 2);
    }
    Ok(gate)
}

/// Fuse top and bottom horizontal gates on one cut before truncating it.
/// Wires in a circuit column are disjoint, so either partner can move next to
/// the other. Resolve matrices on demand; observe once per completed update.
pub fn apply_layer(
    state: &mut ColumnMps,
    columns: usize,
    ops: &[Operation],
    mut matrix: impl FnMut(&Operation) -> DMatrix<Complex64>,
    mut observed: impl FnMut(&ColumnMps),
) -> Result<(), &'static str> {
    if columns != state.columns() {
        return Err("layer width does not match the MPS");
    }
    let wires = columns.checked_mul(2).ok_or("ladder width overflow")?;
    let mut owner = vec![usize::MAX; wires];
    let mut positions = Vec::with_capacity(ops.len());
    for (i, op) in ops.iter().enumerate() {
        for &wire in &op.qubits {
            let slot = owner.get_mut(wire as usize).ok_or("wire outside ladder")?;
            if *slot != usize::MAX {
                return Err("operations in a ladder layer must use disjoint wires");
            }
            *slot = i;
        }
        positions.push(position(op, columns)?);
    }

    for (i, (op, pos)) in ops.iter().zip(positions).enumerate() {
        match pos {
            Position::Single { column, row } => {
                state.apply_one(column, row, &ordered_matrix(op, &mut matrix)?);
            }
            Position::Rung(column) => {
                state.apply_rung(column, &ordered_matrix(op, &mut matrix)?);
            }
            Position::Leg { column, row } => {
                let opposite = (1 - row) * columns + column;
                let partner = owner[opposite];
                if partner != usize::MAX && partner == owner[opposite + 1] {
                    if partner < i {
                        continue; // Already applied with its partner.
                    }
                    let first = ordered_matrix(op, &mut matrix)?;
                    let second = ordered_matrix(&ops[partner], &mut matrix)?;
                    let (top, bottom) = if row == 0 {
                        (&first, &second)
                    } else {
                        (&second, &first)
                    };
                    state.apply_leg_pair(column, top, bottom)?;
                } else {
                    state.apply_leg(column, row, &ordered_matrix(op, &mut matrix)?)?;
                }
            }
        }
        observed(state);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantsim_core::types::Gate;

    fn matrix(op: &Operation) -> DMatrix<Complex64> {
        let (z, o, i) = (Complex64::ZERO, Complex64::ONE, Complex64::I);
        match op.id {
            Gate::X => DMatrix::from_row_slice(2, 2, &[z, o, o, z]),
            Gate::SqrtX => {
                let p = Complex64::new(0.5, 0.5);
                DMatrix::from_row_slice(2, 2, &[p, p.conj(), p.conj(), p])
            }
            Gate::CX | Gate::CY => {
                let mut gate = DMatrix::identity(4, 4);
                gate.swap_rows(2, 3);
                if op.id == Gate::CY {
                    gate[(2, 3)] = -i;
                    gate[(3, 2)] = i;
                }
                gate
            }
            _ => unreachable!(),
        }
    }

    fn op(id: Gate, wires: &[u32]) -> Operation {
        Operation::new(id, wires.to_vec(), vec![])
    }

    #[test]
    fn paired_layer_respects_operand_order_and_reverses_coherently() {
        let mut state = ColumnMps::new(4);
        let prep = [op(Gate::SqrtX, &[1]), op(Gate::X, &[5]), op(Gate::X, &[6])];
        apply_layer(&mut state, 4, &prep, matrix, |_| {}).unwrap();

        // Two asymmetric horizontal gates, separated by a disjoint vertical CX.
        let mut layer = [
            op(Gate::CX, &[1, 0]),
            op(Gate::CX, &[6, 2]),
            op(Gate::CY, &[5, 4]),
        ];
        let mut updates = 0;
        apply_layer(&mut state, 4, &layer, matrix, |_| updates += 1).unwrap();
        assert_eq!(updates, 2);

        let p = Complex64::new(0.5, 0.5);
        let base = (1 << 6) | (1 << 5) | (1 << 4) | (1 << 2);
        for (basis, amp) in state.statevector().into_iter().enumerate() {
            let expected = if basis == base {
                Complex64::I * p
            } else if basis == (base | 3) {
                Complex64::I * p.conj()
            } else {
                Complex64::ZERO
            };
            assert!((amp - expected).norm() < 1e-12, "basis {basis}");
        }

        layer.reverse(); // Exercise bottom-first pairing in the adjoint.
        apply_layer(&mut state, 4, &layer, |op| matrix(op).adjoint(), |_| {}).unwrap();
        apply_layer(&mut state, 4, &prep, |op| matrix(op).adjoint(), |_| {}).unwrap();
        for (basis, amp) in state.statevector().into_iter().enumerate() {
            let expected = if basis == 0 {
                Complex64::ONE
            } else {
                Complex64::ZERO
            };
            assert!((amp - expected).norm() < 1e-12);
        }
    }

    #[test]
    fn invalid_geometry_and_overlapping_layers_fail_before_execution() {
        let mut state = ColumnMps::new(3);
        for invalid in [
            op(Gate::CX, &[1, 2]), // Shares q1 with the first operation.
            op(Gate::CX, &[0, 2]), // Nonadjacent.
            op(Gate::CX, &[2, 3]), // Wraps from top row to bottom.
            op(Gate::CX, &[0, 0]),
            op(Gate::X, &[6]),
        ] {
            let ops = [op(Gate::X, &[1]), invalid];
            assert!(
                apply_layer(&mut state, 3, &ops, |_| unreachable!(), |_| unreachable!()).is_err()
            );
        }
        assert!(apply_layer(&mut state, 2, &[], matrix, |_| {}).is_err());
    }
}
