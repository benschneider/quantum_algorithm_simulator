// Compile the existing experiment unchanged. This sibling of its original
// executable adds read-only validation access to its private tensor storage.
include!("../sycamore_mps/chain.rs");

impl ColumnMps {
    pub fn max_bond(&self) -> usize {
        self.sites.iter().map(DMatrix::ncols).max().unwrap()
    }

    pub fn zero_amplitude(&self) -> Complex64 {
        let mut left = DMatrix::from_element(1, 1, Complex64::ONE);
        for site in &self.sites {
            left = left * site.rows(0, site.nrows() / PHYSICAL);
        }
        left[(0, 0)]
    }

    pub fn norm_squared_at_center(&self) -> f64 {
        self.sites[self.center].norm_squared()
    }
}
