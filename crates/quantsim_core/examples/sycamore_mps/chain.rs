use nalgebra::{
    linalg::SymmetricEigen, DMatrix, DMatrixView, DMatrixViewMut, Dyn, Matrix4, Vector4,
};
use num_complex::Complex64;
use std::mem::size_of;

// A column's physical index is top + 2 * bottom. A site's (left, physical,
// right) tensor is stored as a column-major (4 * left) × right matrix, with
// left the fastest varying index. Both unfoldings thus share the same data.
const PHYSICAL: usize = 4;

fn right_unfold(site: &DMatrix<Complex64>) -> DMatrixView<'_, Complex64> {
    DMatrixView::from_slice(
        site.as_slice(),
        site.nrows() / PHYSICAL,
        PHYSICAL * site.ncols(),
    )
}

pub struct ColumnMps {
    sites: Vec<DMatrix<Complex64>>,
    center: usize,
    discarded_weight: f64,
    discarded_weights: Vec<f64>,
    cap_discarded_weights: Vec<f64>,
    cut_caps: Vec<usize>,
}

impl ColumnMps {
    pub fn new(columns: usize) -> Self {
        Self::with_bond_cap(columns, usize::MAX)
    }

    pub fn with_bond_cap(columns: usize, cap: usize) -> Self {
        assert!(columns > 0 && cap > 0);
        Self::zero(columns, vec![cap; columns - 1])
    }

    /// Set one positive maximum Schmidt rank per cut, in left-to-right order.
    pub fn with_cut_caps(columns: usize, caps: &[usize]) -> Result<Self, &'static str> {
        if columns == 0 || caps.len() != columns - 1 {
            return Err("expected at least one column and one cap per cut");
        }
        if caps.contains(&0) {
            return Err("cut caps must be positive");
        }
        Ok(Self::zero(columns, caps.to_vec()))
    }

    fn zero(columns: usize, cut_caps: Vec<usize>) -> Self {
        let mut zero = DMatrix::zeros(PHYSICAL, 1);
        zero[(0, 0)] = Complex64::ONE;
        Self {
            sites: vec![zero; columns],
            center: 0,
            discarded_weight: 0.0,
            discarded_weights: vec![0.0; cut_caps.len()],
            cap_discarded_weights: vec![0.0; cut_caps.len()],
            cut_caps,
        }
    }

    pub fn columns(&self) -> usize {
        self.sites.len()
    }

    pub fn bonds(&self) -> Vec<usize> {
        self.sites[..self.sites.len() - 1]
            .iter()
            .map(DMatrix::ncols)
            .collect()
    }

    /// Bytes in the retained complex tensor entries, excluding scratch.
    pub fn state_bytes(&self) -> usize {
        self.sites
            .iter()
            .map(|site| site.len() * size_of::<Complex64>())
            .sum()
    }

    pub fn discarded_weight_sum(&self) -> f64 {
        self.discarded_weight
    }

    /// Squared pre-normalization residuals accumulated at each cut, including
    /// rows removed at the QR roundoff threshold and by the Schmidt rank cap.
    pub fn discarded_weights(&self) -> &[f64] {
        &self.discarded_weights
    }

    /// Squared residuals from binding Schmidt rank caps alone, by cut.
    pub fn cap_discarded_weights(&self) -> &[f64] {
        &self.cap_discarded_weights
    }

    /// Move the orthogonality center with thin QR. Outside the center, tensors
    /// are isometric, so a local residual has the same norm as the state's residual.
    fn move_center(&mut self, target: usize) {
        while self.center < target {
            let i = self.center;
            let (q, r) = self.sites[i].clone().qr().unpack();
            let next = &self.sites[i + 1];
            let absorbed = r * right_unfold(next);
            self.sites[i + 1] =
                absorbed.reshape_generic(Dyn(PHYSICAL * q.ncols()), Dyn(next.ncols()));
            self.sites[i] = q;
            self.center += 1;
        }
        while self.center > target {
            let i = self.center;
            let (q, r) = right_unfold(&self.sites[i]).adjoint().qr().unpack();
            self.sites[i - 1] = &self.sites[i - 1] * r.adjoint();
            self.sites[i] = q
                .adjoint()
                .reshape_generic(Dyn(PHYSICAL * q.ncols()), Dyn(self.sites[i].ncols()));
            self.center -= 1;
        }
    }

    fn apply_local(&mut self, column: usize, gate: Matrix4<Complex64>) {
        // A unitary on the physical index preserves either isometry.
        let site = &mut self.sites[column];
        let left = site.nrows() / PHYSICAL;
        for r in 0..site.ncols() {
            for l in 0..left {
                let input = Vector4::from_fn(|p, _| site[(l + left * p, r)]);
                let output = gate * input;
                for p in 0..PHYSICAL {
                    site[(l + left * p, r)] = output[p];
                }
            }
        }
    }

    /// Apply a single-qubit unitary on the top (0) or bottom (1) wire.
    pub fn apply_one(&mut self, column: usize, bit: usize, gate: &DMatrix<Complex64>) {
        assert!(bit < 2 && gate.shape() == (2, 2));
        let local = Matrix4::from_fn(|out, input| {
            if (out ^ input) & (1 << (1 - bit)) == 0 {
                gate[((out >> bit) & 1, (input >> bit) & 1)]
            } else {
                Complex64::ZERO
            }
        });
        self.apply_local(column, local);
    }

    /// The gate on a vertical edge uses |top,bottom⟩ order.
    pub fn apply_rung(&mut self, column: usize, gate: &DMatrix<Complex64>) {
        assert_eq!(gate.shape(), (4, 4));
        let gate_index = |physical| ((physical & 1) << 1) | (physical >> 1);
        self.apply_local(
            column,
            Matrix4::from_fn(|out, input| gate[(gate_index(out), gate_index(input))]),
        );
    }

    /// Apply the horizontal two-qubit gate in |left wire,right wire⟩ order.
    pub fn apply_leg(
        &mut self,
        column: usize,
        bit: usize,
        gate: &DMatrix<Complex64>,
    ) -> Result<(), &'static str> {
        self.apply_legs(column, &[(bit, gate)])
    }

    /// Apply both horizontal gates across a cut before a single factorization.
    /// Each gate uses |left wire,right wire⟩ order on its respective row.
    pub fn apply_leg_pair(
        &mut self,
        column: usize,
        top: &DMatrix<Complex64>,
        bottom: &DMatrix<Complex64>,
    ) -> Result<(), &'static str> {
        self.apply_legs(column, &[(0, top), (1, bottom)])
    }

    fn apply_legs(
        &mut self,
        column: usize,
        gates: &[(usize, &DMatrix<Complex64>)],
    ) -> Result<(), &'static str> {
        assert!(column < self.sites.len() - 1);
        assert!(gates
            .iter()
            .all(|(bit, gate)| *bit < 2 && gate.shape() == (4, 4)));
        self.move_center(self.center.clamp(column, column + 1));
        let right = self.sites[column + 1].ncols();
        let mut pair = &self.sites[column] * right_unfold(&self.sites[column + 1]);
        for &(bit, gate) in gates {
            apply_horizontal_gate(&mut pair, bit, gate);
        }

        let dim = pair.nrows().max(pair.ncols());
        let (q, mut r, permutation) = pair.col_piv_qr().unpack();
        // Column pivoting puts independent directions first. Drop trailing rows
        // only when their combined norm is at the scale of roundoff.
        let cutoff = dim as f64 * f64::EPSILON * r.norm();
        let mut discarded = 0.0;
        let mut rank = r.nrows();
        while rank > 0 {
            let residual = discarded + r.row(rank - 1).norm_squared();
            if residual > cutoff * cutoff {
                break;
            }
            discarded = residual;
            rank -= 1;
        }
        if rank == 0 {
            return Err("two-site state has zero numerical rank");
        }
        let cap = self.cut_caps[column];
        let mut center = if rank > cap {
            let (basis, projected, lost) = truncate_schmidt(r.rows_mut(0, rank), cap)?;
            self.sites[column] = q.columns(0, rank) * basis;
            discarded += lost;
            self.cap_discarded_weights[column] += lost;
            projected
        } else {
            self.sites[column] = q.columns(0, rank).into_owned();
            r.rows(0, rank).into_owned()
        };
        self.discarded_weight += discarded;
        self.discarded_weights[column] += discarded;
        permutation.inv_permute_columns(&mut center);
        self.sites[column + 1] =
            center.reshape_generic(Dyn(PHYSICAL * self.sites[column].ncols()), Dyn(right));
        self.center = column + 1;
        Ok(())
    }

    /// Contract the MPS for comparison only. The dense simulator uses
    /// little-endian wire order, with all top wires followed by all bottom wires.
    pub fn statevector(&self) -> Vec<Complex64> {
        let mut contracted = self.sites[0].clone();
        for site in &self.sites[1..] {
            let rows = PHYSICAL * contracted.nrows();
            contracted =
                (contracted * right_unfold(site)).reshape_generic(Dyn(rows), Dyn(site.ncols()));
        }
        let columns = self.sites.len();
        let mut dense = vec![Complex64::ZERO; contracted.nrows()];
        for (packed, amplitude) in contracted.iter().enumerate() {
            let mut wire_index = 0;
            for col in 0..columns {
                let physical = packed >> (2 * col);
                wire_index |= (physical & 1) << col;
                wire_index |= ((physical >> 1) & 1) << (col + columns);
            }
            dense[wire_index] = *amplitude;
        }
        dense
    }
}

fn apply_horizontal_gate(pair: &mut DMatrix<Complex64>, bit: usize, gate: &DMatrix<Complex64>) {
    let left = pair.nrows() / PHYSICAL;
    let right = pair.ncols() / PHYSICAL;
    let gate = Matrix4::from_fn(|i, j| gate[(i, j)]);
    // Fix both spectator bits and the two virtual indices; the four
    // remaining entries span precisely the physical gate's input space.
    for spectator_left in 0..2 {
        for spectator_right in 0..2 {
            let physical = |k: usize| {
                let a = (spectator_left << (1 - bit)) | ((k >> 1) << bit);
                let b = (spectator_right << (1 - bit)) | ((k & 1) << bit);
                (a, b)
            };
            for r in 0..right {
                for l in 0..left {
                    let index = |k| {
                        let (a, b) = physical(k);
                        (l + left * a, b + PHYSICAL * r)
                    };
                    let input = Vector4::from_fn(|k, _| pair[index(k)]);
                    let output = gate * input;
                    for k in 0..4 {
                        pair[index(k)] = output[k];
                    }
                }
            }
        }
    }
}

/// Return the leading left Schmidt subspace, its normalized coefficients, and
/// the squared norm lost when projecting the pre-normalization state.
fn truncate_schmidt(
    mut center: DMatrixViewMut<'_, Complex64>,
    cap: usize,
) -> Result<(DMatrix<Complex64>, DMatrix<Complex64>, f64), &'static str> {
    let basis = schmidt_basis(center.as_view(), cap)?;
    let mut projected = basis.ad_mul(&center);
    // Reuse the QR buffer for the actual residual. Subtracting two total
    // weights would lose precision for a nearly exact cap.
    center.gemm(-Complex64::ONE, &basis, &projected, Complex64::ONE);
    let discarded = center.norm_squared();
    let norm = projected.norm();
    if !norm.is_finite() || norm == 0.0 {
        return Err("Schmidt projection has invalid norm");
    }
    projected.unscale_mut(norm);
    Ok((basis, projected, discarded))
}

fn schmidt_basis(
    center: DMatrixView<'_, Complex64>,
    cap: usize,
) -> Result<DMatrix<Complex64>, &'static str> {
    let rank = center.nrows();
    // Accumulate only the lower triangle of R R† which the eigensolver reads.
    let mut gram = DMatrix::zeros(rank, rank);
    for column in center.column_iter() {
        gram.hegerc(Complex64::ONE, &column, &column, Complex64::ONE);
    }
    let eig = SymmetricEigen::try_new(gram, f64::EPSILON, 200 * rank)
        .ok_or("Schmidt Gram eigensolver did not converge")?;
    let mut order: Vec<_> = (0..rank).collect();
    order.sort_unstable_by(|&a, &b| eig.eigenvalues[b].total_cmp(&eig.eigenvalues[a]));
    // Squaring singular values can hide the separation between small modes.
    // Use the smaller Gram problem only when the retained subspace is well resolved.
    let gap = eig.eigenvalues[order[cap - 1]] - eig.eigenvalues[order[cap]];
    if gap > f64::EPSILON.sqrt() * eig.eigenvalues[order[0]] {
        return Ok(DMatrix::from_fn(rank, cap, |i, j| {
            eig.eigenvectors[(i, order[j])]
        }));
    }
    drop(eig);
    augmented_schmidt_basis(center, cap)
}

fn augmented_schmidt_basis(
    center: DMatrixView<'_, Complex64>,
    cap: usize,
) -> Result<DMatrix<Complex64>, &'static str> {
    let rank = center.nrows();
    // A thin QR of the adjoint drops the right isometry. For the remaining
    // square factor B, [0 B†; B 0] has eigenvalues ±σ. Its positive eigenvectors'
    // top halves span the left Schmidt directions, including for complex states.
    let adjoint = center.adjoint();
    let factor = if adjoint.nrows() > rank {
        adjoint.qr().unpack_r()
    } else {
        adjoint
    };
    let mut hermitian = DMatrix::zeros(2 * rank, 2 * rank);
    // SymmetricEigen uses only the lower triangle of a Hermitian input.
    hermitian
        .view_mut((rank, 0), (rank, rank))
        .copy_from(&factor);
    drop(factor);
    let eig = SymmetricEigen::try_new(hermitian, f64::EPSILON, 200 * rank)
        .ok_or("Schmidt eigensolver did not converge")?;
    let mut order: Vec<_> = (0..2 * rank).collect();
    order.sort_unstable_by(|&a, &b| eig.eigenvalues[b].total_cmp(&eig.eigenvalues[a]));
    let leading = DMatrix::from_fn(rank, cap, |i, j| eig.eigenvectors[(i, order[j])]);
    Ok(leading.qr().q())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cnot() -> DMatrix<Complex64> {
        let mut gate = DMatrix::identity(4, 4);
        gate.swap_rows(2, 3);
        gate
    }

    // Prepare (3|0> + 4i|1>)/5 so both the phase and the magnitudes are observable.
    fn prepare_control(state: &mut ColumnMps) {
        let a = Complex64::new(0.6, 0.0);
        let b = Complex64::new(0.0, 0.8);
        state.apply_one(0, 0, &DMatrix::from_row_slice(2, 2, &[a, b, b, a]));
    }

    fn assert_control_branches(state: &ColumnMps, excited: usize) {
        for (basis, amplitude) in state.statevector().into_iter().enumerate() {
            let expected = match basis {
                0 => Complex64::new(0.6, 0.0),
                i if i == excited => Complex64::new(0.0, 0.8),
                _ => Complex64::ZERO,
            };
            assert!(
                (amplitude - expected).norm() < 1e-12,
                "basis {basis}: {amplitude}"
            );
        }
    }

    #[test]
    fn undoing_entanglement_recovers_rank_storage_and_relative_phase() {
        let mut state = ColumnMps::new(2);
        let initial_bytes = state.state_bytes();
        prepare_control(&mut state);
        let gate = cnot();
        state.apply_leg(0, 0, &gate).unwrap();
        assert_eq!(state.bonds(), [2]);
        assert_control_branches(&state, 0b0011);

        state.apply_leg(0, 0, &gate).unwrap();
        assert_eq!(state.bonds(), [1]);
        assert_eq!(state.state_bytes(), initial_bytes);
        assert_control_branches(&state, 0b0001);
    }

    #[test]
    fn rung_and_leg_controls_follow_row_major_wire_order() {
        let mut state = ColumnMps::new(4);
        prepare_control(&mut state);
        let gate = cnot();
        state.apply_rung(0, &gate); // q0 -> q4, within one column
        assert_eq!(state.bonds(), [1, 1, 1]);
        state.apply_leg(0, 1, &gate).unwrap(); // q4 -> q5
        assert_control_branches(&state, 0b0011_0001);

        state.apply_leg(1, 1, &gate).unwrap(); // q5 -> q6
        assert_control_branches(&state, 0b0111_0001);
        state.apply_leg(2, 1, &gate).unwrap(); // q6 -> q7
        assert_control_branches(&state, 0b1111_0001);
        state.apply_leg(0, 0, &gate).unwrap(); // q0 -> q1, across the first cut again
        assert_eq!(state.bonds(), [2, 2, 2]);
        assert_control_branches(&state, 0b1111_0011);
    }

    #[test]
    fn paired_legs_preserve_asymmetric_controls_and_complex_phases() {
        let mut state = ColumnMps::new(2);
        prepare_control(&mut state);
        let a = Complex64::new(0.96, 0.0);
        let b = Complex64::new(0.0, 0.28);
        state.apply_one(0, 1, &DMatrix::from_row_slice(2, 2, &[a, b, b, a]));
        let initial = state.statevector();

        let mut controlled_y = cnot();
        controlled_y[(3, 2)] = Complex64::I;
        controlled_y[(2, 3)] = -Complex64::I;
        state.apply_leg_pair(0, &cnot(), &controlled_y).unwrap();
        let mut expected = vec![Complex64::ZERO; 16];
        expected[0] = Complex64::new(0.576, 0.0);
        expected[3] = Complex64::new(0.0, 0.768);
        expected[12] = Complex64::new(-0.168, 0.0);
        expected[15] = Complex64::new(0.0, -0.224);
        for (actual, expected) in state.statevector().iter().zip(expected) {
            assert!((actual - expected).norm() < 1e-12);
        }
        state.apply_leg_pair(0, &cnot(), &controlled_y).unwrap();
        for (actual, initial) in state.statevector().iter().zip(initial) {
            assert!((actual - initial).norm() < 1e-12);
        }
    }

    #[test]
    fn paired_legs_avoid_truncating_transient_intercolumn_entanglement() {
        let s = Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0);
        let h = DMatrix::from_row_slice(2, 2, &[s, s, s, -s]);
        let gate = cnot();
        let mut paired = ColumnMps::with_bond_cap(2, 1);
        let mut separate = ColumnMps::with_bond_cap(2, 1);
        for state in [&mut paired, &mut separate] {
            for column in 0..2 {
                state.apply_one(column, 0, &h);
                state.apply_rung(column, &gate);
            }
        }
        // Each column holds a Bell pair. Together the two horizontal CNOTs
        // permute the four equal branches back into the same product of columns.
        let initial = paired.statevector();
        paired.apply_leg_pair(0, &gate, &gate).unwrap();
        assert_eq!(paired.bonds(), [1]);
        assert!(paired.discarded_weight_sum() < 1e-25);
        for (actual, initial) in paired.statevector().iter().zip(initial) {
            assert!((actual - initial).norm() < 1e-12);
        }
        separate.apply_leg(0, 0, &gate).unwrap();
        assert!((separate.discarded_weight_sum() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn cut_caps_constrain_only_their_own_cut_and_attribute_its_loss() {
        assert!(ColumnMps::with_cut_caps(0, &[]).is_err());
        assert!(ColumnMps::with_cut_caps(1, &[]).unwrap().bonds().is_empty());
        assert!(ColumnMps::with_cut_caps(3, &[2]).is_err());
        assert!(ColumnMps::with_cut_caps(3, &[1, 0]).is_err());
        let mut state = ColumnMps::with_cut_caps(3, &[1, 2]).unwrap();
        prepare_control(&mut state);
        state.apply_leg(0, 0, &cnot()).unwrap();
        let s = Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0);
        state.apply_one(1, 1, &DMatrix::from_row_slice(2, 2, &[s, s, s, -s]));
        state.apply_leg(1, 1, &cnot()).unwrap();
        assert_eq!(state.bonds(), [1, 2]);
        let weights = state.discarded_weights();
        assert!((weights[0] - 0.36).abs() < 1e-12);
        assert!(weights[1] < 1e-25);
        assert_eq!(weights.iter().sum::<f64>(), state.discarded_weight_sum());
        let cap_losses = state.cap_discarded_weights();
        assert!((cap_losses[0] - 0.36).abs() < 1e-12);
        assert_eq!(cap_losses[1], 0.0);
    }

    #[test]
    fn qr_roundoff_is_attributed_to_the_cut_even_without_a_rank_cap() {
        let mut state = ColumnMps::new(2);
        let a = Complex64::ONE;
        let b = Complex64::new(0.0, 1e-16);
        state.apply_one(0, 0, &DMatrix::from_row_slice(2, 2, &[a, b, b, a]));
        state.apply_leg(0, 0, &cnot()).unwrap();
        assert_eq!(state.bonds(), [1]);
        assert!((state.discarded_weights()[0] - 1e-32).abs() < 1e-44);
        assert_eq!(state.discarded_weights()[0], state.discarded_weight_sum());
        assert_eq!(state.cap_discarded_weights(), [0.0]);
    }

    // Entangle both rows across one bond, rotating the right bottom Schmidt
    // vectors to |+> and |->. The four Schmidt weights are the products of
    // (0.6², 0.8²) and (0.96², 0.28²).
    fn prepare_four_schmidt_modes(state: &mut ColumnMps) {
        prepare_control(state);
        state.apply_leg(0, 0, &cnot()).unwrap();
        let a = Complex64::new(0.96, 0.0);
        let b = Complex64::new(0.0, 0.28);
        state.apply_one(0, 1, &DMatrix::from_row_slice(2, 2, &[a, b, b, a]));
        let s = Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0);
        let h = DMatrix::from_row_slice(2, 2, &[s, s, s, -s]);
        let gate = DMatrix::identity(2, 2).kronecker(&h) * cnot();
        state.apply_leg(0, 1, &gate).unwrap();
    }

    #[test]
    fn binding_cap_keeps_the_leading_complex_modes_and_normalizes() {
        let mut state = ColumnMps::with_bond_cap(2, 2);
        prepare_four_schmidt_modes(&mut state);
        assert_eq!(state.bonds(), [2]);
        assert!((state.discarded_weight_sum() - 0.28_f64.powi(2)).abs() < 1e-12);
        for (i, amplitude) in state.statevector().into_iter().enumerate() {
            // The kept bottom state is |0+>, with the original top pair intact.
            let expected = match i {
                0 | 8 => Complex64::new(0.6, 0.0),
                3 | 11 => Complex64::new(0.0, 0.8),
                _ => Complex64::ZERO,
            } * std::f64::consts::FRAC_1_SQRT_2;
            assert!((amplitude - expected).norm() < 1e-12, "basis {i}");
        }
    }

    #[test]
    fn nonbinding_cap_preserves_exact_path_and_roundoff_accounting() {
        let mut exact = ColumnMps::new(2);
        let mut capped = ColumnMps::with_bond_cap(2, 4);
        let mut profiled = ColumnMps::with_cut_caps(2, &[4]).unwrap();
        prepare_four_schmidt_modes(&mut exact);
        prepare_four_schmidt_modes(&mut capped);
        prepare_four_schmidt_modes(&mut profiled);
        assert_eq!(exact.bonds(), [4]);
        assert_eq!(capped.bonds(), exact.bonds());
        assert_eq!(capped.statevector(), exact.statevector());
        assert_eq!(capped.discarded_weight_sum(), exact.discarded_weight_sum());
        assert_eq!(profiled.statevector(), capped.statevector());
        assert_eq!(profiled.discarded_weights(), capped.discarded_weights());
    }

    #[test]
    fn weak_schmidt_modes_remain_resolvable_below_gram_precision() {
        let fourier = DMatrix::from_fn(4, 4, |i, j| {
            Complex64::from_polar(0.5, std::f64::consts::FRAC_PI_2 * (i * j) as f64)
        });
        // The 1e-10 component must survive a rank-two projection even though
        // its weight falls below the numerical precision of the density matrix.
        let mut modes = DMatrix::from_diagonal_element(4, 4, Complex64::ONE);
        modes[(1, 1)] = Complex64::new(0.0, 1e-10);
        modes[(2, 2)] = Complex64::new(2e-11, 0.0);
        modes[(3, 3)] = Complex64::new(0.0, 1e-12);
        let mut center = &fourier * &modes * &fourier;
        let (basis, projected, lost) = truncate_schmidt(center.as_view_mut(), 2).unwrap();
        modes[(2, 2)] = Complex64::ZERO;
        modes[(3, 3)] = Complex64::ZERO;
        let ideal = &fourier * modes * &fourier;
        assert!((basis * projected - ideal).norm() < 1e-14);
        assert!((lost - 4.01e-22).abs() < 1e-26);
    }
}
