use nalgebra::Complex;
use quantsim_core::core::circuit::StepSnapshot;
use quantsim_core::core::circuit::{Circuit, RunOptions};

/// The `Simulation` struct is responsible for running quantum circuit
/// simulations and storing their results. It acts as an interface to the
/// core `qcsim` library's simulation capabilities.
///
/// This struct holds the final state vector of the simulation and a list
/// of snapshots taken at each step of the circuit execution, which can be
/// used for visualization and analysis.
pub struct Simulation {
    pub state_vector: Option<Vec<Complex<f32>>>,
    pub snapshots: Vec<StepSnapshot>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            state_vector: None,
            snapshots: Vec::new(),
        }
    }

    /// Runs a quantum circuit simulation.
    ///
    /// This function takes a `Circuit` as input, executes it using the
    /// `qcsim` core library, and stores the resulting state vector and
    /// snapshots.
    ///
    /// The `RunOptions` are configured to take snapshots at each step,
    /// allowing for a detailed view of the quantum state evolution.
    pub fn run_simulation(&mut self, circuit: &mut Circuit) {
        // If the sub-circuit has no steps, we should just show the initial state vector.
        if circuit.steps.is_empty() {
            let num_qubits = circuit.num_qubits;
            let mut initial_state = vec![Complex::new(0.0, 0.0); 1 << num_qubits];
            initial_state[0] = Complex::new(1.0, 0.0);
            self.state_vector = Some(initial_state);
            return;
        }

        let opts = RunOptions {
            shots: None,
            snapshot_state_per_step: true, // We need the last state
            rng_seed: None,
        };
        if let Some(initial_state) = &circuit.initial_state {
            let initial_state_vec = nalgebra::DVector::from_iterator(initial_state.len(), initial_state.iter().cloned());
            log::debug!(
                "Simulation::run_simulation - Circuit initial_state: {}",
                quantsim_core::core::formatters::get_pretty_print_state_vector(&initial_state_vec, circuit.num_qubits)
            );
        } else {
            log::debug!("Simulation::run_simulation - Circuit initial_state: default |00...0⟩");
        }
        let result = circuit.run(&opts);
        self.state_vector = result.snapshots.last().and_then(|s| {
            s.state.as_ref().map(|sv| {
                sv.iter()
                    .map(|c| Complex::new(c.re, c.im)) // No f64 conversion needed
                    .collect()
            })
        });
        self.snapshots = result.snapshots;
    }

    /// Calculates the probability of measuring a specific qubit in the |1> state.
    ///
    /// This function iterates through the state vector and sums the
    /// probabilities of all basis states where the specified qubit is in the |1> state.
    ///
    /// # Arguments
    ///
    /// * `qubit_index` - The index of the qubit for which to calculate the probability.
    ///
    /// # Returns
    ///
    /// An `Option` containing the probability as an `f32`, or `None` if the
    /// state vector is not available.
    pub fn get_qubit_probabilities(&self, qubit_index: usize) -> Option<f32> {
        // Changed to f32
        self.state_vector.as_ref().map(|state_vector| {
            let num_qubits = (state_vector.len() as f32).log2() as usize; // Changed to f32
            let mut prob_one = 0.0;
            for (i, state) in state_vector.iter().enumerate() {
                if (i >> (num_qubits - 1 - qubit_index)) & 1 == 1 {
                    prob_one += state.norm_sqr();
                }
            }
            prob_one
        })
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}
