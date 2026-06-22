//! Integration tests for AHP mathematical calculations.

/// Common setup and utilities for tests.
pub mod common;
use approx::assert_relative_eq;

/// Tests the calculation endpoint using a perfectly consistent matrix.
#[tokio::test]
async fn test_calculate_perfectly_consistent() {
    // Initialize the test context with an in-memory database and test server
    let ctx = common::TestContext::new().await;
    
    // Retrieve the perfectly consistent golden matrix
    let golden = common::golden_data::perfectly_consistent_matrix();

    // Send the matrix to the calculation endpoint
    let response = ctx
        .server
        .post("/api/ahp/calculate")
        .json(&serde_json::json!({
            "matrix": golden.matrix
        }))
        .await;

    // Assert that the response status is OK
    response.assert_status_ok();

    // Parse the JSON response
    let json = response.json::<serde_json::Value>();

    // Extract the consistency ratio from the response
    let cr = json["consistency_ratio"].as_f64().unwrap();
    
    // Assert that the calculated CR matches the expected CR
    assert_relative_eq!(cr, golden.expected_cr, epsilon = 1e-4);

    // Extract the priority vector (weights) from the response
    let weights = json["priority_vector"].as_array().unwrap();
    
    // Assert that the number of weights matches expectations
    assert_eq!(weights.len(), golden.expected_weights.len());

    // Iterate through and verify each weight against the golden data
    for (i, w) in weights.iter().enumerate() {
        assert_relative_eq!(
            w.as_f64().unwrap(),
            golden.expected_weights[i],
            epsilon = 1e-4
        );
    }
}

/// Tests the calculation endpoint using an invalid, highly inconsistent matrix.
#[tokio::test]
async fn test_calculate_invalid_consistency() {
    // Initialize the test context with an in-memory database and test server
    let ctx = common::TestContext::new().await;
    
    // Retrieve the matrix with invalid consistency
    let golden = common::golden_data::invalid_consistency_matrix();

    // Send the invalid matrix to the calculation endpoint
    let response = ctx
        .server
        .post("/api/ahp/calculate")
        .json(&serde_json::json!({
            "matrix": golden.matrix
        }))
        .await;

    // Assert that the response status is OK (the endpoint still calculates it)
    response.assert_status_ok();

    // Parse the JSON response
    let json = response.json::<serde_json::Value>();

    // Extract the consistency ratio from the response
    let cr = json["consistency_ratio"].as_f64().unwrap();
    
    // Assert that the CR is appropriately large, indicating inconsistency
    assert!(cr > 0.10);
}
