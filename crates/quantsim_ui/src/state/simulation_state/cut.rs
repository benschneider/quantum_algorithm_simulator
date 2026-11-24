use super::SimulationState;
use quantsim_core::circuit::Circuit;
use quantsim_core::core::schmidt::{cut_readout, SchmidtReadout, MAX_SCHMIDT_QUBITS};
use quantsim_core::mps_ladder::{cut_bound, CutOrder};
use quantsim_core::BasisIndex;

#[derive(Debug)]
pub(crate) struct CutInspection {
    pub step: usize,
    pub order: CutOrder,
    pub left_sites: usize,
    pub rank_bound: Result<BasisIndex, String>,
    pub crossing_gates: Vec<usize>,
    pub dense: Option<Result<SchmidtReadout, String>>,
}

impl SimulationState {
    pub(crate) fn inspect_cut(
        &mut self,
        circuit: &Circuit,
        step: usize,
        order: CutOrder,
        left_sites: usize,
    ) -> &CutInspection {
        let needs_update = self
            .cut
            .as_ref()
            .is_none_or(|cut| (cut.step, cut.order, cut.left_sites) != (step, order, left_sites));
        if needs_update {
            let wires = circuit.num_qubits;
            let (rank_bound, crossing_gates) = match cut_bound(circuit, step, order, left_sites) {
                Ok(bound) => (Ok(bound.rank), bound.crossing_gates),
                Err(message) => {
                    // An imported entangled input can still be measured and its gates drawn.
                    let crossing = step
                        .checked_sub(1)
                        .and_then(|i| circuit.steps.get(i))
                        .into_iter()
                        .flatten()
                        .enumerate()
                        .filter(|(_, op)| order.crosses(&op.qubits, wires, left_sites))
                        .map(|(index, _)| index)
                        .collect();
                    (Err(message), crossing)
                }
            };
            let dense = self
                .quantum_state
                .as_ref()
                .filter(|_| wires <= MAX_SCHMIDT_QUBITS)
                .map(|state| cut_readout(state.state_vector.as_slice(), wires, order, left_sites));
            self.cut = Some(CutInspection {
                step,
                order,
                left_sites,
                rank_bound,
                crossing_gates,
                dense,
            });
        }
        self.cut
            .as_ref()
            .expect("the selected cut has been analyzed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{messages::Message, QCSimApp};
    use nalgebra::Complex;

    #[test]
    fn imported_state_keeps_its_dense_readout_when_the_gate_bound_is_unavailable() {
        let mut app = QCSimApp::default();
        app.handle_message(Message::LoadTemplate("bell.json".into()));
        app.handle_message(Message::DeleteGate(0, 0)); // Leave only CX.
        let a = std::f64::consts::FRAC_1_SQRT_2;
        app.handle_message(Message::ApplyInitialStateFromEditor(
            [a, 0.0, 0.0, a].map(|x| Complex::new(x, 0.0)).to_vec(),
        ));
        for (step, rank) in [(0, 2), (2, 1)] {
            app.handle_message(Message::SelectTimestep(step));
            let analysis = app.state.simulation_state.inspect_cut(
                &app.state.circuit_state.circuit,
                step,
                CutOrder::WireOrder,
                1,
            );
            assert!(analysis.rank_bound.is_err());
            assert_eq!(
                analysis.dense.as_ref().unwrap().as_ref().unwrap().rank,
                rank
            );
            assert_eq!(analysis.crossing_gates.len(), usize::from(step == 2));
        }
        app.handle_message(Message::ResetInitialState);
        assert!(app.state.simulation_state.cut.is_none());
        let analysis = app.state.simulation_state.inspect_cut(
            &app.state.circuit_state.circuit,
            2,
            CutOrder::WireOrder,
            1,
        );
        assert_eq!(analysis.rank_bound, Ok(2u8.into()));
        assert!(analysis.dense.is_none());
    }
}
