use crate::mps_ladder::CutOrder;
use nalgebra::{linalg::SymmetricEigen, Complex, DMatrix, DVector};

/// Bounds the balanced reduced density matrix to 256 × 256 on desktop and web.
pub const MAX_SCHMIDT_QUBITS: usize = 16;

/// A numerical readout of a pure state's bipartite Schmidt spectrum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SchmidtReadout {
    /// Number of Schmidt coefficients larger than 1e-6 times the state norm.
    pub rank: usize,
    /// Von Neumann entropy in bits, using all positive normalized eigenvalues.
    pub entropy_bits: f64,
}

/// Inspect the partition after the first `left_sites` in the chosen MPS order.
/// Wire q0 is the least significant basis bit. For paired columns, a left
/// site includes the corresponding wires from both halves of the register.
pub fn cut_readout(
    state: &[Complex<f32>],
    wires: usize,
    order: CutOrder,
    left_sites: usize,
) -> Result<SchmidtReadout, String> {
    if wires > MAX_SCHMIDT_QUBITS {
        return Err(format!(
            "Dense cut readout supports up to {MAX_SCHMIDT_QUBITS} qubits."
        ));
    }
    let sites = order.site_count(wires)?;
    if left_sites == 0 || left_sites >= sites {
        return Err("The cut must have at least one site on each side.".into());
    }
    if state.len() != 1 << wires {
        return Err("The dense state length does not match the wire count.".into());
    }
    let low_bits = (1usize << left_sites) - 1;
    let left_mask = match order {
        CutOrder::WireOrder => low_bits,
        CutOrder::PairedColumns => low_bits | (low_bits << sites),
    };
    let right_mask = (state.len() - 1) ^ left_mask;
    let (small, large) = if left_mask.count_ones() <= right_mask.count_ones() {
        (left_mask, right_mask)
    } else {
        (right_mask, left_mask)
    };

    // Trace out the larger side. Each assignment there contributes the outer
    // product of one conditional amplitude vector on the smaller side.
    let offsets: Vec<_> = (0..state.len()).filter(|bits| bits & large == 0).collect();
    let dim = offsets.len();
    let mut reduced = DMatrix::zeros(dim, dim);
    let mut column = DVector::zeros(dim);
    for outer in (0..state.len()).filter(|bits| bits & small == 0) {
        for (entry, &inner) in column.iter_mut().zip(&offsets) {
            let amplitude = state[outer | inner];
            if !amplitude.re.is_finite() || !amplitude.im.is_finite() {
                return Err("Dense cut readout requires finite amplitudes.".into());
            }
            *entry = Complex::new(f64::from(amplitude.re), f64::from(amplitude.im));
        }
        // Only the lower Hermitian triangle is needed by the eigensolver.
        reduced.hegerc(Complex::ONE, &column, &column, Complex::ONE);
    }
    let trace = reduced.trace().re;
    if trace == 0.0 {
        return Err("Dense cut readout requires a nonzero state.".into());
    }
    let eig = SymmetricEigen::try_new(reduced, f64::EPSILON, 200 * dim)
        .ok_or("The Schmidt eigensolver did not converge.")?;
    let mut rank = 0;
    let mut entropy_bits = 0.0;
    for &weight in &eig.eigenvalues {
        // Normalize away f32 norm drift. Tiny negative eigenvalues can occur
        // at numerical zero. The rank threshold is on the Schmidt amplitude
        // (1e-6 for f32 input), hence 1e-12 on its squared magnitude here.
        let p = (weight / trace).clamp(0.0, 1.0);
        if p > 1e-12 {
            rank += 1;
        }
        if p > 0.0 {
            entropy_bits -= p * p.log2();
        }
    }
    Ok(SchmidtReadout { rank, entropy_bits })
}

#[cfg(test)]
mod tests {
    use super::*;
    use CutOrder::{PairedColumns, WireOrder};

    #[test]
    fn paired_columns_put_both_rows_of_a_rung_on_the_same_side() {
        // Entangle q0 with q3 on a 2×3 ladder; include a relative phase
        // to check conjugation in the reduced density matrix.
        let mut state = vec![Complex::ZERO; 64];
        state[0] = Complex::new(0.6, 0.0);
        state[1 | (1 << 3)] = Complex::new(0.0, 0.8);
        for cut in [1, 2] {
            let same_side = cut_readout(&state, 6, PairedColumns, cut).unwrap();
            assert_eq!(same_side.rank, 1);
            assert!(same_side.entropy_bits.abs() < 1e-12);
        }
        let split = cut_readout(&state, 6, WireOrder, 1).unwrap();
        assert_eq!(split.rank, 2);
        assert!((split.entropy_bits - 0.9426831893).abs() < 1e-7);
        state.swap(1 | (1 << 3), 1 | (1 << 5));
        for cut in [1, 2] {
            assert_eq!(cut_readout(&state, 6, PairedColumns, cut).unwrap(), split);
        }
    }

    #[test]
    fn numerical_rank_resolves_weak_weights_above_the_f32_readout_threshold() {
        let mut state = vec![Complex::ZERO; 4];
        state[0] = Complex::ONE;
        for (coefficient, rank) in [(1e-5, 2), (1e-7, 1)] {
            state[3] = Complex::new(0.0, coefficient);
            let readout = cut_readout(&state, 2, WireOrder, 1).unwrap();
            assert_eq!(readout.rank, rank);
            assert!(readout.entropy_bits > 0.0);
        }
    }

    #[test]
    fn rotated_schmidt_modes_survive_rounding_to_f32() {
        // Three orthogonal Fourier modes on each side, with Schmidt weights
        // 1/2, 3/10 and 1/5. Every amplitude can be complex and nonzero.
        let weights: [f64; 3] = [0.5, 0.3, 0.2];
        let state: Vec<_> = (0..256)
            .map(|basis| {
                let (a, b) = (basis & 15, basis >> 4);
                let amplitude: Complex<f64> = weights
                    .iter()
                    .enumerate()
                    .map(|(k, w)| {
                        let angle = std::f64::consts::TAU * (k * a + (k + 1) * b) as f64 / 16.0;
                        Complex::from_polar(w.sqrt() / 16.0, angle)
                    })
                    .sum();
                Complex::new(amplitude.re as f32, amplitude.im as f32)
            })
            .collect();
        let measured = cut_readout(&state, 8, WireOrder, 4).unwrap();
        let entropy: f64 = weights.iter().map(|p| -p * p.log2()).sum();
        assert_eq!(measured.rank, 3);
        assert!((measured.entropy_bits - entropy).abs() < 1e-7);
    }

    #[test]
    fn malformed_cuts_and_invalid_states_return_errors() {
        let state = vec![Complex::ZERO; 4];
        for (wires, order, cut) in [
            (2, WireOrder, 0),
            (2, WireOrder, 2),
            (3, PairedColumns, 1),
            (usize::MAX, WireOrder, 1),
            (6, PairedColumns, 1),
        ] {
            assert!(cut_readout(&state, wires, order, cut).is_err());
        }
        assert!(cut_readout(&state, 2, WireOrder, 1).is_err());
        let mut invalid = state;
        invalid[2] = Complex::new(f32::NAN, 1.0);
        assert!(cut_readout(&invalid, 2, WireOrder, 1).is_err());
    }
}
