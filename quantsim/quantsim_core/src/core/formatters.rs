use nalgebra::{Complex, DMatrix, DVector};
use nalgebra_sparse::CsrMatrix;

/// Formats a sparse matrix for pretty printing, separating real and imaginary parts.
pub fn format_matrix(matrix: &CsrMatrix<Complex<f32>>) -> String {
    let mut s = String::new();
    let dense = DMatrix::from(matrix);
    if dense.nrows() == 0 || dense.ncols() == 0 {
        return "[]\n".to_string();
    }

    s.push_str("Real Part:\n");
    for r in 0..dense.nrows() {
        s.push_str("  ");
        for c in 0..dense.ncols() {
            s.push_str(&format!("{: >10.4} ", dense[(r, c)].re));
        }
        s.push('\n');
    }

    s.push_str("\nImaginary Part:\n");
    for r in 0..dense.nrows() {
        s.push_str("  ");
        for c in 0..dense.ncols() {
            s.push_str(&format!("{: >10.4} ", dense[(r, c)].im));
        }
        s.push('\n');
    }
    s
}

/// Formats a sub-vector for pretty printing, showing local basis states.
pub fn format_sub_vector(vector: &DVector<Complex<f32>>, sub_qubits: &[usize]) -> String {
    let mut s = String::new();
    let n_sub_qubits = sub_qubits.len();
    if n_sub_qubits == 0 {
        // Should not happen for any real gate, but handle gracefully.
        return format!("{:?}", vector);
    }
    s.push_str("  Local Index | Binary |       Real |  Imaginary\n");
    s.push_str("  --------------------------------------------------\n");
    for i in 0..vector.len() {
        let amp = vector[i];
        s.push_str(&format!(
            "  {:>11} | {:>6} | {: >10.4} | {: >10.4}\n",
            i,
            format_args!("{:0width$b}", i, width = n_sub_qubits),
            amp.re,
            amp.im
        ));
    }
    s
}

/// Returns a formatted string representing the state vector for debugging.
///
/// The output shows each basis state and its corresponding complex amplitude.
/// e.g., "|01> = Complex { re: 0.0, im: 0.0 }"
pub fn get_pretty_print_state_vector(
    state_vector: &DVector<Complex<f32>>,
    num_qubits: usize,
) -> String {
    let mut output = String::new();
    for (i, amp) in state_vector.iter().enumerate() {
        if amp.norm_sqr() > 1e-6 {
            output.push_str(&format!(
                "         |{:0width$b}>: {}\n",
                i,
                format_complex(*amp),
                width = num_qubits
            ));
        }
    }
    output
}

pub fn format_final_state(state_vector: &DVector<Complex<f32>>, num_qubits: usize) -> String {
    let mut parts = Vec::new();
    for (i, amp) in state_vector.iter().enumerate() {
        if amp.norm_sqr() > 1e-6 {
            parts.push(format!(
                "{:+.2}|{:0width$b}>",
                amp.re,
                i,
                width = num_qubits
            ));
        }
    }
    parts.join(" ")
}

fn format_complex(c: Complex<f32>) -> String {
    let is_real_zero = c.re.abs() < 1e-6;
    let is_imag_zero = c.im.abs() < 1e-6;

    match (is_real_zero, is_imag_zero) {
        (false, true) => format!("{: >5.2}", c.re),      // Real part only
        (true, false) => format!("{: >5.2}i", c.im),     // Imaginary part only
        (true, true) => "  0".to_string(),             // Both are zero
        (false, false) => format!("{:+.2}{:+.2}i", c.re, c.im), // Both are non-zero
    }
}

/// Formats a matrix-vector multiplication for logging, showing the operation in a more intuitive,
/// human-readable format.
pub fn format_transform(
    u: &CsrMatrix<Complex<f32>>,
    v_in: &DVector<Complex<f32>>,
    v_out: &DVector<Complex<f32>>,
) -> String {
    let mut s = String::new();
    let dense_u = DMatrix::from(u);
    let n_rows = dense_u.nrows();

    for r in 0..n_rows {
        // Matrix part
        s.push_str(" |");
        for c in 0..dense_u.ncols() {
            s.push_str(&format!("{} ", format_complex(dense_u[(r, c)])));
        }
        s.push_str("|");

        // Input vector part
        s.push_str(&format!(" · {}", format_complex(v_in[r])));

        // Output vector part
        s.push_str(&format!(" = {}\n", format_complex(v_out[r])));
    }
    s
}