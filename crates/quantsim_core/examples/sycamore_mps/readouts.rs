use num_complex::Complex64;
use std::io::{self, Write};

/// Computing these from the verification vectors gives exact Z-basis readouts
/// of each represented state without sampling. Wire q is bit q of a basis index.
pub struct LocalReadouts {
    p1: Vec<f64>,
    p11: Vec<f64>,
}

impl LocalReadouts {
    pub fn measure(amplitudes: &[Complex64], wires: usize, links: &[[usize; 2]]) -> Self {
        assert_eq!(amplitudes.len(), 1 << wires);
        let masks: Vec<usize> = links.iter().map(|&[a, b]| (1 << a) | (1 << b)).collect();
        let mut p1 = vec![0.0; wires];
        let mut p11 = vec![0.0; links.len()];
        let mut total = 0.0;
        for (basis, amplitude) in amplitudes.iter().enumerate() {
            let weight = amplitude.norm_sqr();
            total += weight;
            for (wire, p) in p1.iter_mut().enumerate() {
                if basis & (1 << wire) != 0 {
                    *p += weight;
                }
            }
            for (&mask, p) in masks.iter().zip(&mut p11) {
                if basis & mask == mask {
                    *p += weight;
                }
            }
        }
        // The sweep checks the norms independently. Divide by the measured norm
        // so small floating-point drift does not bias the reported probabilities.
        for p in p1.iter_mut().chain(&mut p11) {
            *p /= total;
        }
        Self { p1, p11 }
    }

    fn pair_values(&self, edge: usize, [a, b]: [usize; 2]) -> [f64; 3] {
        let joint = self.p11[edge];
        let zz = 1.0 - 2.0 * (self.p1[a] + self.p1[b]) + 4.0 * joint;
        let connected = 4.0 * (joint - self.p1[a] * self.p1[b]);
        [joint, zz, connected]
    }

    pub fn write_comparison(
        &self,
        out: &mut impl Write,
        dense: &Self,
        links: &[[usize; 2]],
        round: usize,
        cap: &str,
    ) -> io::Result<()> {
        for (wire, (&d, &m)) in dense.p1.iter().zip(&self.p1).enumerate() {
            writeln!(out, "{round},{cap},p1,{wire},,{d:.15e},{m:.15e}")?;
        }
        for (edge, &[a, b]) in links.iter().enumerate() {
            let exact = dense.pair_values(edge, [a, b]);
            let approx = self.pair_values(edge, [a, b]);
            for ((name, d), m) in ["p11", "zz", "connected_zz"]
                .into_iter()
                .zip(exact)
                .zip(approx)
            {
                writeln!(out, "{round},{cap},{name},{a},{b},{d:.15e},{m:.15e}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_excitation_and_anticorrelated_bell_pair_use_patch_wire_order() {
        // q1 is always 1; q0 and q2 are an anticorrelated Bell pair.
        let mut state = vec![Complex64::ZERO; 8];
        state[0b011] = Complex64::ONE;
        state[0b110] = Complex64::I;
        let links = [[2, 0], [1, 2]];
        let readouts = LocalReadouts::measure(&state, 3, &links);
        assert_eq!(readouts.p1, [0.5, 1.0, 0.5]);
        assert_eq!(readouts.pair_values(0, links[0]), [0.0, -1.0, -1.0]);
        assert_eq!(readouts.pair_values(1, links[1]), [0.5, 0.0, 0.0]);
    }

    #[test]
    fn deterministic_product_has_raw_zz_but_no_connected_correlation() {
        let mut state = vec![Complex64::ZERO; 4];
        state[3] = Complex64::ONE;
        let readouts = LocalReadouts::measure(&state, 2, &[[0, 1]]);
        assert_eq!(readouts.p1, [1.0, 1.0]);
        assert_eq!(readouts.pair_values(0, [0, 1]), [1.0, 1.0, 0.0]);
    }
}
