//! This module contains a simple test for the `ehttp` crate, specifically testing header manipulation.

use ehttp::Request;

/// The main function of the test executable.
fn main() {
    // Create a new POST request to localhost with dummy payload.
    let mut req = Request::post("http://localhost", vec![1, 2, 3]);

    // Retain all headers except for the content-type header.
    req.headers
        .headers
        .retain(|(k, _)| k.to_lowercase() != "content-type");

    // Insert a new content-type header specifying application/json.
    req.headers.insert("Content-Type", "application/json");

    // Print the headers after the modification.
    println!("After retain and insert: {:?}", req.headers.headers);
}
