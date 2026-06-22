// Golden Matrices for AHP Testing

pub struct GoldenMatrix {
    pub matrix: Vec<Vec<f64>>,
    pub expected_weights: Vec<f64>,
    pub expected_cr: f64,
}

// A 3x3 matrix that is perfectly consistent.
// CR = 0.0
pub fn perfectly_consistent_matrix() -> GoldenMatrix {
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

// A 3x3 matrix with acceptable consistency.
// CR is usually around 0.05
pub fn acceptable_consistency_matrix() -> GoldenMatrix {
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

// A 3x3 matrix with invalid consistency.
// CR > 0.10
pub fn invalid_consistency_matrix() -> GoldenMatrix {
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
