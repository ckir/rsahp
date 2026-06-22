import os

path = "backend/tests/test_documents.rs"
with open(path, "r") as f:
    content = f.read()

content = content.replace('.parse().unwrap(),', ',')

with open(path, "w") as f:
    f.write(content)

print("Files patched.")
