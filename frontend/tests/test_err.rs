fn main() {
    let json = "Document not found";
    let v: Result<serde_json::Value, _> = serde_json::from_str(json);
    println!("{:?}", v);
}
