use ehttp::Request;

fn main() {
    let mut req = Request::post("http://localhost", vec![1, 2, 3]);
    req.headers
        .headers
        .retain(|(k, _)| k.to_lowercase() != "content-type");
    req.headers.insert("Content-Type", "application/json");
    println!("After retain and insert: {:?}", req.headers.headers);
}
