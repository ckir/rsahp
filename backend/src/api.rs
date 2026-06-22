use axum::{routing::post, Json, Router};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

use crate::ahp::{aggregate_aij, aggregate_aip, calculate_priorities, AhpResult};

pub fn router() -> Router {
    Router::new()
        .route("/calculate", post(calculate_handler))
        .route("/aggregate/aij", post(aggregate_aij_handler))
        .route("/aggregate/aip", post(aggregate_aip_handler))
}

#[derive(Deserialize)]
pub struct CalculateRequest {
    pub matrix: Vec<Vec<f64>>,
}

#[derive(Serialize)]
pub struct CalculateResponse {
    pub priority_vector: Vec<f64>,
    pub lambda_max: f64,
    pub consistency_index: f64,
    pub consistency_ratio: f64,
}

impl From<AhpResult> for CalculateResponse {
    fn from(res: AhpResult) -> Self {
        Self {
            priority_vector: res.priority_vector.iter().copied().collect(),
            lambda_max: res.lambda_max,
            consistency_index: res.consistency_index,
            consistency_ratio: res.consistency_ratio,
        }
    }
}

async fn calculate_handler(
    Json(payload): Json<CalculateRequest>,
) -> Result<Json<CalculateResponse>, String> {
    let n = payload.matrix.len();
    if n == 0 {
        return Err("Empty matrix".to_string());
    }
    let mut dmatrix = DMatrix::zeros(n, n);
    for (i, row) in payload.matrix.into_iter().enumerate() {
        if row.len() != n {
            return Err("Matrix must be square".to_string());
        }
        for (j, val) in row.into_iter().enumerate() {
            dmatrix[(i, j)] = val;
        }
    }

    let result = calculate_priorities(&dmatrix)?;
    Ok(Json(result.into()))
}

#[derive(Deserialize)]
pub struct AggregateAijRequest {
    pub matrices: Vec<Vec<Vec<f64>>>,
}

#[derive(Serialize)]
pub struct AggregateAijResponse {
    pub consensus_matrix: Vec<Vec<f64>>,
}

async fn aggregate_aij_handler(
    Json(payload): Json<AggregateAijRequest>,
) -> Result<Json<AggregateAijResponse>, String> {
    let mut dmatrices = Vec::new();
    for matrix_data in payload.matrices {
        let n = matrix_data.len();
        if n == 0 {
            continue;
        }
        let mut dmatrix = DMatrix::zeros(n, n);
        for (i, row) in matrix_data.into_iter().enumerate() {
            if row.len() != n {
                return Err("All matrices must be square".to_string());
            }
            for (j, val) in row.into_iter().enumerate() {
                dmatrix[(i, j)] = val;
            }
        }
        dmatrices.push(dmatrix);
    }

    let consensus = aggregate_aij(&dmatrices)?;
    let n = consensus.nrows();
    let mut response_matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            response_matrix[i][j] = consensus[(i, j)];
        }
    }
    Ok(Json(AggregateAijResponse {
        consensus_matrix: response_matrix,
    }))
}

#[derive(Deserialize)]
pub struct AggregateAipRequest {
    pub vectors: Vec<Vec<f64>>,
}

#[derive(Serialize)]
pub struct AggregateAipResponse {
    pub consensus_vector: Vec<f64>,
}

async fn aggregate_aip_handler(
    Json(payload): Json<AggregateAipRequest>,
) -> Result<Json<AggregateAipResponse>, String> {
    let mut dvectors = Vec::new();
    for vec_data in payload.vectors {
        let n = vec_data.len();
        if n == 0 {
            continue;
        }
        let mut dvector = DVector::zeros(n);
        for (i, val) in vec_data.into_iter().enumerate() {
            dvector[i] = val;
        }
        dvectors.push(dvector);
    }

    let consensus = aggregate_aip(&dvectors)?;
    let response_vector = consensus.iter().copied().collect();
    Ok(Json(AggregateAipResponse {
        consensus_vector: response_vector,
    }))
}
