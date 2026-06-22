pub mod common;
use approx::assert_relative_eq;

#[tokio::test]
async fn test_calculate_perfectly_consistent() {
    let ctx = common::TestContext::new().await;
    let golden = common::golden_data::perfectly_consistent_matrix();

    let response = ctx.server.post("/api/ahp/calculate")
        .json(&serde_json::json!({
            "matrix": golden.matrix
        }))
        .await;

    response.assert_status_ok();
    
    let json = response.json::<serde_json::Value>();
    
    // Assert CR
    let cr = json["consistency_ratio"].as_f64().unwrap();
    assert_relative_eq!(cr, golden.expected_cr, epsilon = 1e-4);

    // Assert Weights
    let weights = json["priority_vector"].as_array().unwrap();
    assert_eq!(weights.len(), golden.expected_weights.len());
    
    for (i, w) in weights.iter().enumerate() {
        assert_relative_eq!(w.as_f64().unwrap(), golden.expected_weights[i], epsilon = 1e-4);
    }
}

#[tokio::test]
async fn test_calculate_invalid_consistency() {
    let ctx = common::TestContext::new().await;
    let golden = common::golden_data::invalid_consistency_matrix();

    let response = ctx.server.post("/api/ahp/calculate")
        .json(&serde_json::json!({
            "matrix": golden.matrix
        }))
        .await;

    response.assert_status_ok();
    
    let json = response.json::<serde_json::Value>();
    
    // Assert CR is large
    let cr = json["consistency_ratio"].as_f64().unwrap();
    assert!(cr > 0.10);
}
