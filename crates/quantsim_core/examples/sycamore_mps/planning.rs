use crate::{budget, layers, ColumnMps};
use nalgebra::DMatrix;
use num_complex::Complex64;
use quantsim_core::types::Operation;

pub struct BondPlan {
    pub caps: Vec<usize>,
    pub calibration_loss: Vec<f64>,
    pub pilot_peak_tensor_bytes: usize,
}

/// Profile the forward circuit once from the zero state. Only the cut
/// diagnostics survive the pilot, so its tensors never coexist with the replay.
pub fn calibrate(
    columns: usize,
    base_cap: usize,
    budget_bytes: usize,
    steps: &[Vec<Operation>],
    mut matrix: impl FnMut(&Operation) -> DMatrix<Complex64>,
) -> Result<BondPlan, &'static str> {
    let base = budget::base_caps(columns, base_cap)?;
    let minimum = budget::tensor_bound(&base).ok_or("base tensor bound overflows usize")?;
    if minimum > budget_bytes {
        return Err("tensor budget is below the base cap payload bound");
    }
    let (calibration_loss, pilot_peak_tensor_bytes) = {
        let mut pilot = ColumnMps::with_cut_caps(columns, &base)?;
        let mut peak = pilot.state_bytes();
        for layer in steps {
            layers::apply_layer(&mut pilot, columns, layer, &mut matrix, |state| {
                peak = peak.max(state.state_bytes())
            })?;
        }
        // A bigger cap cannot recover directions removed by the QR roundoff rule.
        (pilot.cap_discarded_weights().to_vec(), peak)
    };
    let caps = budget::plan_caps(&calibration_loss, base_cap, budget_bytes)?;
    Ok(BondPlan {
        caps,
        calibration_loss,
        pilot_peak_tensor_bytes,
    })
}
