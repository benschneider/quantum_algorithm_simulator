use num_complex::Complex64;
use std::mem::size_of;

// Each ladder column holds two qubits in one four-dimensional physical index.
const PHYSICAL: usize = 4;
const SITE_BYTES_PER_BOND_PAIR: usize = PHYSICAL * size_of::<Complex64>();

fn physical_cap(columns: usize, cut: usize) -> usize {
    let smaller_side = (cut + 1).min(columns - cut - 1);
    if smaller_side >= (usize::BITS / 2) as usize {
        usize::MAX
    } else {
        1usize << (2 * smaller_side)
    }
}

/// Apply a common cap, clipped to the Hilbert-space dimension on each side of a cut.
pub fn base_caps(columns: usize, base_cap: usize) -> Result<Vec<usize>, &'static str> {
    if columns == 0 || base_cap == 0 {
        return Err("column count and base bond cap must be positive");
    }
    let cuts = columns - 1;
    let mut caps = Vec::new();
    caps.try_reserve_exact(cuts)
        .map_err(|_| "could not allocate one bond cap per cut")?;
    caps.extend((0..cuts).map(|cut| base_cap.min(physical_cap(columns, cut))));
    Ok(caps)
}

/// Maximum bytes of retained complex tensor entries at these caps. An empty slice
/// describes one site with both boundary bonds equal to one.
pub fn tensor_bound(caps: &[usize]) -> Option<usize> {
    let mut pairs = 0usize;
    let mut left = 1usize;
    for right in caps.iter().copied().chain(std::iter::once(1)) {
        if right == 0 {
            return None;
        }
        pairs = pairs.checked_add(left.checked_mul(right)?)?;
        left = right;
    }
    pairs.checked_mul(SITE_BYTES_PER_BOND_PAIR)
}

/// Spend a retained-tensor budget on cuts with positive accumulated discarded
/// weight. At each step, grow the highest-loss cut that can still increase,
/// breaking ties left to right. This is a greedy plan from a prior run.
pub fn plan_caps(
    losses: &[f64],
    base_cap: usize,
    budget_bytes: usize,
) -> Result<Vec<usize>, &'static str> {
    if losses.iter().any(|&loss| !loss.is_finite() || loss < 0.0) {
        return Err("discarded weights must be finite and nonnegative");
    }
    let columns = losses.len().checked_add(1).ok_or("column count overflow")?;
    let mut caps = base_caps(columns, base_cap)?;
    let required = tensor_bound(&caps).ok_or("base tensor payload size overflow")?;
    let mut remaining = budget_bytes
        .checked_sub(required)
        .ok_or("tensor budget is below the base-cap payload")?;
    let mut priority: Vec<_> = (0..losses.len()).filter(|&cut| losses[cut] > 0.0).collect();
    priority.sort_unstable_by(|&a, &b| losses[b].total_cmp(&losses[a]).then(a.cmp(&b)));

    // Raising a neighboring cap can make a previously saturated cut grow again.
    loop {
        let mut grew = false;
        for &cut in &priority {
            let left = if cut == 0 { 1 } else { caps[cut - 1] };
            let right = caps.get(cut + 1).copied().unwrap_or(1);
            let limit = physical_cap(columns, cut).min(PHYSICAL.saturating_mul(left.min(right)));
            // One added rank enlarges exactly its two incident tensors. The
            // current payload already includes at least this many bytes.
            let bytes_per_rank = SITE_BYTES_PER_BOND_PAIR * (left + right);
            let growth = (limit - caps[cut]).min(remaining / bytes_per_rank);
            if growth > 0 {
                caps[cut] += growth;
                remaining -= growth * bytes_per_rank;
                grew = true;
                break;
            }
        }
        if !grew {
            return Ok(caps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_column_profile_spends_the_budget_on_the_middle_cut() {
        let losses = [0.0, 0.0, 0.250, 0.561, 0.101, 0.0, 0.0];
        let caps = plan_caps(&losses, 32, 585 * 1024).unwrap();
        assert_eq!(base_caps(8, 32).unwrap(), [4, 16, 32, 32, 32, 16, 4]);
        assert_eq!(caps, [4, 16, 32, 128, 32, 16, 4]);
        assert_eq!(tensor_bound(&caps), Some(598_528));
    }

    #[test]
    fn tie_break_and_partial_growth_charge_exactly_the_affected_sites() {
        let losses = [0.0, 1.0, 0.0, 1.0, 0.0];
        let base = base_caps(6, 8).unwrap();
        let bytes = tensor_bound(&base).unwrap();
        let increment = 64 * (4 + 8);
        assert_eq!(plan_caps(&losses, 8, bytes + increment - 1).unwrap(), base);
        let grown = plan_caps(&losses, 8, bytes + increment).unwrap();
        assert_eq!(grown, [4, 9, 8, 8, 4]);
        assert_eq!(tensor_bound(&grown), Some(bytes + increment));
    }

    #[test]
    fn obeys_neighbor_ranks_and_leaves_zero_loss_cuts_at_base() {
        let losses = [0.0, 0.0, 1.0, 3.0, 1.0, 0.0, 0.0];
        let caps = plan_caps(&losses, 1, usize::MAX).unwrap();
        assert_eq!(caps, [1, 1, 4, 16, 4, 1, 1]);
        assert_eq!(plan_caps(&[0.0; 7], 1, usize::MAX).unwrap(), [1; 7]);
        assert_eq!(
            base_caps(8, usize::MAX).unwrap(),
            [4, 16, 64, 256, 64, 16, 4]
        );
    }

    #[test]
    fn handles_boundaries_and_rejects_invalid_inputs() {
        assert_eq!(tensor_bound(&[]), Some(64));
        assert!(plan_caps(&[], 1, 64).unwrap().is_empty());
        assert!(plan_caps(&[], 1, 63).is_err());
        assert!(base_caps(0, 1).is_err());
        assert!(base_caps(usize::MAX, 1).is_err());
        assert!(plan_caps(&[1.0], 0, 1024).is_err());
        for invalid in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(plan_caps(&[invalid], 1, 1024).is_err());
        }
        assert!(tensor_bound(&[0]).is_none());
        assert!(tensor_bound(&[usize::MAX]).is_none());
        assert!(plan_caps(&[0.0; 128], usize::MAX, usize::MAX).is_err());
    }
}
