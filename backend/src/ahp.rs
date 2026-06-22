use nalgebra::{DMatrix, DVector};

/// Represents the result of an AHP calculation for a single matrix.
#[derive(Debug, Clone)]
pub struct AhpResult {
    pub priority_vector: DVector<f64>,
    pub lambda_max: f64,
    pub consistency_index: f64,
    pub consistency_ratio: f64,
}

/// Random Index (RI) table for matrices up to size 10.
const RANDOM_INDEX: &[f64] = &[
    0.0, 0.0, 0.0, 0.58, 0.90, 1.12, 1.24, 1.32, 1.41, 1.45, 1.49,
];

/// Calculates the priority vector and consistency metrics using the Row Geometric Mean Method.
pub fn calculate_priorities(matrix: &DMatrix<f64>) -> Result<AhpResult, String> {
    let n = matrix.nrows();
    if n != matrix.ncols() {
        return Err("Matrix must be square".to_string());
    }
    if n == 0 {
        return Err("Matrix cannot be empty".to_string());
    }
    if n == 1 {
        return Ok(AhpResult {
            priority_vector: DVector::from_element(1, 1.0),
            lambda_max: 1.0,
            consistency_index: 0.0,
            consistency_ratio: 0.0,
        });
    }

    // Row Geometric Mean
    let mut geom_means = DVector::zeros(n);
    for i in 0..n {
        let mut prod = 1.0;
        for j in 0..n {
            prod *= matrix[(i, j)];
        }
        geom_means[i] = prod.powf(1.0 / (n as f64));
    }

    // Normalize to get Priority Vector
    let sum: f64 = geom_means.iter().sum();
    let priority_vector = &geom_means / sum;

    // Calculate lambda_max: Aw = lambda * w
    let aw = matrix * &priority_vector;
    let mut lambda_max = 0.0;
    for i in 0..n {
        lambda_max += aw[i] / priority_vector[i];
    }
    lambda_max /= n as f64;

    let consistency_index = (lambda_max - n as f64) / ((n - 1) as f64);

    let ri = if n < RANDOM_INDEX.len() {
        RANDOM_INDEX[n]
    } else {
        // Fallback for larger matrices (approximation)
        1.98 * (1.0 - (n as f64 - 1.0) / (n as f64 * (n as f64 + 1.0) / 2.0))
    };

    let consistency_ratio = if ri == 0.0 {
        0.0
    } else {
        consistency_index / ri
    };

    Ok(AhpResult {
        priority_vector,
        lambda_max,
        consistency_index,
        consistency_ratio,
    })
}

/// Aggregation of Individual Judgments (AIJ)
/// Uses the geometric mean of individual matrices to form a consensus matrix.
pub fn aggregate_aij(matrices: &[DMatrix<f64>]) -> Result<DMatrix<f64>, String> {
    if matrices.is_empty() {
        return Err("No matrices provided".to_string());
    }
    let n = matrices[0].nrows();
    let num_matrices = matrices.len() as f64;

    let mut consensus = DMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            let mut prod = 1.0;
            for matrix in matrices {
                if matrix.nrows() != n || matrix.ncols() != n {
                    return Err("All matrices must be of the same size".to_string());
                }
                prod *= matrix[(i, j)];
            }
            consensus[(i, j)] = prod.powf(1.0 / num_matrices);
        }
    }
    Ok(consensus)
}

/// Aggregation of Individual Priorities (AIP)
/// Uses the geometric mean of individual priority vectors to form a consensus vector.
pub fn aggregate_aip(vectors: &[DVector<f64>]) -> Result<DVector<f64>, String> {
    if vectors.is_empty() {
        return Err("No vectors provided".to_string());
    }
    let n = vectors[0].len();
    let num_vectors = vectors.len() as f64;

    let mut consensus = DVector::zeros(n);
    for i in 0..n {
        let mut prod = 1.0;
        for vec in vectors {
            if vec.len() != n {
                return Err("All vectors must be of the same size".to_string());
            }
            prod *= vec[i];
        }
        consensus[i] = prod.powf(1.0 / num_vectors);
    }

    // Normalize
    let sum: f64 = consensus.iter().sum();
    Ok(consensus / sum)
}
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::{dmatrix, dvector};

    #[test]
    fn test_ahp_3x3() {
        // Saaty's classic example
        // C1, C2, C3
        let matrix = dmatrix![
            1.0, 1.0/3.0, 5.0;
            3.0, 1.0,     7.0;
            1.0/5.0, 1.0/7.0, 1.0
        ];

        let result = calculate_priorities(&matrix).unwrap();

        // Check priority vector roughly equals standard AHP approximations
        // For this matrix, w ≈ [0.279, 0.649, 0.072]
        assert_relative_eq!(result.priority_vector[0], 0.279, epsilon = 0.01);
        assert_relative_eq!(result.priority_vector[1], 0.649, epsilon = 0.01);
        assert_relative_eq!(result.priority_vector[2], 0.072, epsilon = 0.01);

        // Check lambda_max >= 3
        assert!(result.lambda_max >= 3.0);
        // CR should be acceptable
        assert!(result.consistency_ratio < 0.1);
    }

    #[test]
    fn test_aij() {
        let m1 = dmatrix![
            1.0, 2.0;
            0.5, 1.0
        ];
        let m2 = dmatrix![
            1.0, 8.0;
            0.125, 1.0
        ];

        let consensus = aggregate_aij(&[m1, m2]).unwrap();
        // sqrt(2 * 8) = 4
        // sqrt(0.5 * 0.125) = sqrt(0.0625) = 0.25
        assert_relative_eq!(consensus[(0, 1)], 4.0, epsilon = 0.001);
        assert_relative_eq!(consensus[(1, 0)], 0.25, epsilon = 0.001);
    }

    #[test]
    fn test_aip() {
        let v1 = dvector![0.8, 0.2];
        let v2 = dvector![0.2, 0.8];

        let consensus = aggregate_aip(&[v1, v2]).unwrap();
        // Geometric mean of both is sqrt(0.16) = 0.4
        // Normalized: [0.5, 0.5]
        assert_relative_eq!(consensus[0], 0.5, epsilon = 0.001);
        assert_relative_eq!(consensus[1], 0.5, epsilon = 0.001);
    }
}
