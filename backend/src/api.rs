//! Module for AHP-related API endpoints.
//! This module defines the routes and handlers for calculating priorities,
//! and aggregating judgments and priorities.

use axum::{Json, Router, routing::post};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use crate::ahp::{AhpResult, aggregate_aij, aggregate_aip, calculate_priorities};

/// Configures and returns the router for AHP API endpoints.
pub fn router() -> Router {
    // Create a new router
    Router::new()
        // Route for calculating priorities
        .route("/calculate", post(calculate_handler))
        // Route for AIJ aggregation
        .route("/aggregate/aij", post(aggregate_aij_handler))
        // Route for AIP aggregation
        .route("/aggregate/aip", post(aggregate_aip_handler))
}

/// Request payload for AHP priority calculation.
#[derive(Deserialize)]
pub struct CalculateRequest {
    /// A square matrix represented as a 2D vector.
    pub matrix: Vec<Vec<f64>>,
}

/// Response payload containing the result of AHP priority calculation.
#[derive(Serialize)]
pub struct CalculateResponse {
    /// The computed priority vector.
    pub priority_vector: Vec<f64>,
    /// The principal eigenvalue (lambda max).
    pub lambda_max: f64,
    /// The consistency index (CI).
    pub consistency_index: f64,
    /// The consistency ratio (CR).
    pub consistency_ratio: f64,
}

/// Conversion from `AhpResult` to `CalculateResponse`.
impl From<AhpResult> for CalculateResponse {
    fn from(res: AhpResult) -> Self {
        // Map fields from AhpResult to CalculateResponse
        Self {
            // Convert DVector to Vec
            priority_vector: res.priority_vector.iter().copied().collect(),
            lambda_max: res.lambda_max,
            consistency_index: res.consistency_index,
            consistency_ratio: res.consistency_ratio,
        }
    }
}

/// Handler for the `/calculate` endpoint.
/// Calculates the priority vector and consistency metrics for a given matrix.
async fn calculate_handler(
    Json(payload): Json<CalculateRequest>,
) -> Result<Json<CalculateResponse>, String> {
    // Get the size of the provided matrix
    let n = payload.matrix.len();
    // Validate if the matrix is not empty
    if n == 0 {
        return Err("Empty matrix".to_string());
    }
    // Initialize a new nalgebra DMatrix
    let mut dmatrix = DMatrix::zeros(n, n);
    // Populate the DMatrix with the values from the payload
    for (i, row) in payload.matrix.into_iter().enumerate() {
        // Check if the row has the correct length
        if row.len() != n {
            return Err("Matrix must be square".to_string());
        }
        for (j, val) in row.into_iter().enumerate() {
            dmatrix[(i, j)] = val;
        }
    }

    // Call the underlying AHP calculation function
    let result = calculate_priorities(&dmatrix)?;
    // Return the result as a JSON response
    Ok(Json(result.into()))
}

/// Request payload for AIJ aggregation.
#[derive(Deserialize)]
pub struct AggregateAijRequest {
    /// A list of square matrices, each represented as a 2D vector.
    pub matrices: Vec<Vec<Vec<f64>>>,
}

/// Response payload containing the consensus matrix from AIJ aggregation.
#[derive(Serialize)]
pub struct AggregateAijResponse {
    /// The calculated consensus matrix.
    pub consensus_matrix: Vec<Vec<f64>>,
}

/// Handler for the `/aggregate/aij` endpoint.
/// Performs Aggregation of Individual Judgments.
async fn aggregate_aij_handler(
    Json(payload): Json<AggregateAijRequest>,
) -> Result<Json<AggregateAijResponse>, String> {
    // Initialize a vector to hold the parsed nalgebra matrices
    let mut dmatrices = Vec::new();
    // Iterate through each matrix in the request payload
    for matrix_data in payload.matrices {
        // Get the size of the current matrix
        let n = matrix_data.len();
        if n == 0 {
            // Skip empty matrices
            continue;
        }
        // Initialize a new DMatrix
        let mut dmatrix = DMatrix::zeros(n, n);
        // Populate the DMatrix with data
        for (i, row) in matrix_data.into_iter().enumerate() {
            // Ensure the matrix is square
            if row.len() != n {
                return Err("All matrices must be square".to_string());
            }
            for (j, val) in row.into_iter().enumerate() {
                dmatrix[(i, j)] = val;
            }
        }
        // Add the parsed matrix to the vector
        dmatrices.push(dmatrix);
    }

    // Perform the AIJ aggregation
    let consensus = aggregate_aij(&dmatrices)?;
    let n = consensus.nrows();
    // Prepare the response matrix representation
    let mut response_matrix = vec![vec![0.0; n]; n];
    // Copy the values from the consensus matrix to the response
    for i in 0..n {
        for j in 0..n {
            response_matrix[i][j] = consensus[(i, j)];
        }
    }
    // Return the response as JSON
    Ok(Json(AggregateAijResponse {
        consensus_matrix: response_matrix,
    }))
}

/// Request payload for AIP aggregation.
#[derive(Deserialize)]
pub struct AggregateAipRequest {
    /// A list of priority vectors.
    pub vectors: Vec<Vec<f64>>,
}

/// Response payload containing the consensus vector from AIP aggregation.
#[derive(Serialize)]
pub struct AggregateAipResponse {
    /// The calculated consensus vector.
    pub consensus_vector: Vec<f64>,
}

/// Handler for the `/aggregate/aip` endpoint.
/// Performs Aggregation of Individual Priorities.
async fn aggregate_aip_handler(
    Json(payload): Json<AggregateAipRequest>,
) -> Result<Json<AggregateAipResponse>, String> {
    // Initialize a vector to hold the parsed nalgebra vectors
    let mut dvectors = Vec::new();
    // Iterate through each vector in the request payload
    for vec_data in payload.vectors {
        // Get the size of the current vector
        let n = vec_data.len();
        if n == 0 {
            // Skip empty vectors
            continue;
        }
        // Initialize a new DVector
        let mut dvector = DVector::zeros(n);
        // Populate the DVector with data
        for (i, val) in vec_data.into_iter().enumerate() {
            dvector[i] = val;
        }
        // Add the parsed vector to the vector list
        dvectors.push(dvector);
    }

    // Perform the AIP aggregation
    let consensus = aggregate_aip(&dvectors)?;
    // Convert the DVector into a standard Vec
    let response_vector = consensus.iter().copied().collect();
    // Return the response as JSON
    Ok(Json(AggregateAipResponse {
        consensus_vector: response_vector,
    }))
}
