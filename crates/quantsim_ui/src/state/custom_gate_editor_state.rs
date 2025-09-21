// crates/qcsim-egui/src/state/custom_gate_editor_state.rs
use nalgebra::{Complex, DMatrix, SVD};
use quantsim_core::core::endian_utils::is_unitary;
use quantsim_core::core::gates::Gate;

/// Represents the selected tab in the custom gate editor.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CustomGateEditorTab {
    Real,
    Imaginary,
}

/// Represents the state of the custom gate editor.
#[derive(Debug, Clone)]
pub struct CustomGateEditorState {
    /// The ID of the gate being edited (e.g., "CUSTOM1").
    pub gate_id: Option<Gate>,
    /// Temporary string representations of the matrix for the UI.
    /// Each element is a tuple of (real_str, imag_str).
    pub matrix_str: DMatrix<(String, String)>,
    /// Holds any validation or parsing error message.
    pub error_message: Option<String>,
    /// Flag to indicate if the editor window should be open.
    pub is_open: bool,
    /// Currently selected tab (Real or Imaginary)
    pub selected_tab: CustomGateEditorTab,
}

impl Default for CustomGateEditorState {
    fn default() -> Self {
        Self {
            gate_id: None,
            matrix_str: DMatrix::from_vec(0, 0, vec![]),
            error_message: None,
            is_open: false,
            selected_tab: CustomGateEditorTab::Real,
        }
    }
}

impl CustomGateEditorState {
    /// Creates a new `CustomGateEditorState` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses the string representation of the matrix and validates it.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed matrix, or an error message if
    /// parsing or validation fails.
    pub fn parse_and_validate_matrix(&mut self) -> Result<DMatrix<Complex<f32>>, String> {
        let rows = self.matrix_str.nrows();
        let cols = self.matrix_str.ncols();
        let mut parsed_matrix = DMatrix::from_element(rows, cols, Complex::new(0.0, 0.0));

        self.error_message = None; // Clear previous errors

        for r in 0..rows {
            for c in 0..cols {
                let (real_str, imag_str) = &self.matrix_str[(r, c)];
                let real_val = real_str.parse::<f32>();
                let imag_val = imag_str.parse::<f32>();

                match (real_val, imag_val) {
                    (Ok(re), Ok(im)) => {
                        parsed_matrix[(r, c)] = Complex::new(re, im);
                    }
                    (Err(_), _) => {
                        self.error_message = Some(format!(
                            "Invalid real part at row {}, column {}: '{}'",
                            r + 1,
                            c + 1,
                            real_str
                        ));
                        return Err(self.error_message.clone().unwrap());
                    }
                    (_, Err(_)) => {
                        self.error_message = Some(format!(
                            "Invalid imaginary part at row {}, column {}: '{}'",
                            r + 1,
                            c + 1,
                            imag_str
                        ));
                        return Err(self.error_message.clone().unwrap());
                    }
                }
            }
        }

        let (is_unitary_val, unitarity_error_val) = is_unitary(&parsed_matrix);
        if !is_unitary_val {
            self.error_message = Some(format!(
                "Matrix is not unitary! Error: {:.2e}",
                unitarity_error_val
            ));
            return Err(self.error_message.clone().unwrap());
        }

        Ok(parsed_matrix)
    }

    /// Converts the current matrix to a unitary matrix using SVD.
    pub fn make_unitary(&mut self) {
        let rows = self.matrix_str.nrows();
        let cols = self.matrix_str.ncols();
        let mut current_matrix = DMatrix::from_element(rows, cols, Complex::new(0.0, 0.0));

        for r in 0..rows {
            for c in 0..cols {
                let (real_str, imag_str) = &self.matrix_str[(r, c)];
                if let (Ok(re), Ok(im)) = (real_str.parse::<f32>(), imag_str.parse::<f32>()) {
                    current_matrix[(r, c)] = Complex::new(re, im);
                } else {
                    // If parsing fails, reset to identity or some default
                    current_matrix = DMatrix::identity(rows, cols);
                    break;
                }
            }
        }

        // SVD takes (matrix, thin, compute_uv)
        let svd = SVD::new(current_matrix, true, true);
        let u = svd.u.unwrap();
        let v_t = svd.v_t.unwrap();
        let unitary_matrix = u * v_t;

        for r in 0..rows {
            for c in 0..cols {
                let complex_val = unitary_matrix[(r, c)];
                self.matrix_str[(r, c)] = (complex_val.re.to_string(), complex_val.im.to_string());
            }
        }
        self.error_message = None; // Clear error after making unitary
    }
}
