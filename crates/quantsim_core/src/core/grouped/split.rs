use super::{Factor, FactorState, GroupedState, Wire};
use num_complex::Complex64;

impl GroupedState {
    pub(super) fn split_singletons(&mut self, index: usize) {
        while let Some(single) = self.factors[index].split_singleton() {
            self.factors.push(single);
        }
    }
}

impl Factor {
    fn split_singleton(&mut self) -> Option<Factor> {
        let FactorState::Group(values) = &self.state else {
            return None;
        };
        // Each wire defines a 2 x N matrix, with one row for each bit value.
        // A largest-amplitude pivot lies in a nonzero row and bounds the
        // magnitude of the other row's candidate multiplier by one.
        let pivot = (0..values.len())
            .max_by(|&a, &b| values[a].norm_sqr().total_cmp(&values[b].norm_sqr()))?;
        for bit in 0..self.wires.len() {
            let mask = 1 << bit;
            let pivot_bit = pivot & mask;
            let row = (0..values.len()).filter(|i| i & mask == pivot_bit);
            let ratio = values[pivot ^ mask] / values[pivot];
            // Require the second row to reproduce every stored complex entry.
            // Even a tiny nonzero residual keeps the group together.
            if !row.clone().all(|i| values[i] * ratio == values[i ^ mask]) {
                continue;
            }
            let scale = 1.0_f64.hypot(ratio.norm());
            let mut pair = [Complex64::from(1.0 / scale), ratio / scale];
            if pivot_bit != 0 {
                pair.swap(0, 1);
            }
            // The pivot branch of the detached wire is real and positive.
            // Its original phase stays in the remaining amplitudes.
            let mut rest = row.map(|i| values[i] * scale);
            self.state = if self.wires.len() == 2 {
                FactorState::Single(Wire::from_factor([rest.next()?, rest.next()?]))
            } else {
                FactorState::Group(rest.collect())
            };
            return Some(Factor {
                wires: vec![self.wires.remove(bit)],
                state: FactorState::Single(Wire::from_factor(pair)),
            });
        }
        None
    }
}
