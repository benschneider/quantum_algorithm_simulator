use nalgebra::{Complex, DMatrix, SVD};
use quantsim_core::core::endian_utils::is_unitary;

/// Represents the selected tab in the custom gate editor.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CustomGateEditorTab {
    Real,
    Imaginary,
}

/// Represents the state of the custom gate editor.
#[derive(Debug, Clone)]
pub struct CustomGateEditorState {
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

    fn parse_matrix(&self) -> Result<DMatrix<Complex<f32>>, String> {
        let (rows, cols) = self.matrix_str.shape();
        let mut parsed = DMatrix::zeros(rows, cols);
        for r in 0..rows {
            for c in 0..cols {
                let (real, imag) = &self.matrix_str[(r, c)];
                let parse = |text: &str, part: &str| {
                    text.trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| {
                            format!(
                                "Enter a finite {part} value at row {}, column {}.",
                                r + 1,
                                c + 1
                            )
                        })
                };
                parsed[(r, c)] = Complex::new(parse(real, "real")?, parse(imag, "imaginary")?);
            }
        }
        Ok(parsed)
    }

    pub fn parse_and_validate_matrix(&self) -> Result<DMatrix<Complex<f32>>, String> {
        let matrix = self.parse_matrix()?;
        let (unitary, error) = is_unitary(&matrix);
        if unitary {
            Ok(matrix)
        } else {
            Err(format!("Matrix is not unitary (error {error:.2e})."))
        }
    }

    /// Project the draft onto a unitary matrix while preserving invalid input for correction.
    pub fn make_unitary(&mut self) {
        let matrix = match self.parse_matrix() {
            Ok(matrix) if !matrix.is_empty() => matrix,
            Ok(_) => return,
            Err(error) => {
                self.error_message = Some(error);
                return;
            }
        };
        let Some(svd) = SVD::try_new(matrix, true, true, f32::EPSILON, 100) else {
            self.error_message = Some("Could not compute a unitary matrix.".into());
            return;
        };
        let unitary = svd.u.unwrap() * svd.v_t.unwrap();
        self.matrix_str = unitary.map(|z| (z.re.to_string(), z.im.to_string()));
        self.error_message = None;
    }
}
