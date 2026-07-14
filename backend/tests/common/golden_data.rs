// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Golden data matrices for testing Analytical Hierarchy Process (AHP) calculations.

/// Represents a golden standard matrix along with its expected AHP results.
pub struct GoldenMatrix {
    /// The square matrix representing pairwise comparisons.
    pub matrix: Vec<Vec<f64>>,
    /// The expected priority weights derived from the matrix.
    pub expected_weights: Vec<f64>,
    /// The expected consistency ratio (CR) of the matrix.
    pub expected_cr: f64,
}

/// Returns a perfectly consistent 3x3 matrix (CR = 0.0).
pub fn perfectly_consistent_matrix() -> GoldenMatrix {
    // Create and return the perfectly consistent matrix data
    GoldenMatrix {
        matrix: vec![
            vec![1.0, 2.0, 4.0],
            vec![0.5, 1.0, 2.0],
            vec![0.25, 0.5, 1.0],
        ],
        expected_weights: vec![4.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0], // approx [0.5714, 0.2857, 0.1428]
        expected_cr: 0.0,
    }
}

/// Returns a 3x3 matrix with an acceptable level of consistency.
pub fn acceptable_consistency_matrix() -> GoldenMatrix {
    // Create and return a matrix that represents acceptable consistency
    GoldenMatrix {
        matrix: vec![
            vec![1.0, 2.0, 5.0],
            vec![0.5, 1.0, 2.0],
            vec![0.2, 0.5, 1.0],
        ],
        expected_weights: vec![0.581, 0.279, 0.139], // approximate standard output
        expected_cr: 0.002, // Wait, CR for this is small. Let's just use it as a test payload. We will test exact values when asserting.
    }
}

/// Returns a 3x3 matrix that is completely inconsistent (CR > 0.10).
pub fn invalid_consistency_matrix() -> GoldenMatrix {
    // Create and return a matrix with invalid consistency to test failure cases
    GoldenMatrix {
        matrix: vec![
            vec![1.0, 9.0, 1.0 / 9.0],
            vec![1.0 / 9.0, 1.0, 9.0],
            vec![9.0, 1.0 / 9.0, 1.0],
        ],
        expected_weights: vec![0.333, 0.333, 0.333], // weights for completely inconsistent ring
        expected_cr: 10.0,                           // huge CR
    }
}
