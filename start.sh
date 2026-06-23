#!/usr/bin/env bash
set -e

echo "Building rsahp..."
cargo build --bin backend
cargo build --bin frontend

if ! pgrep -x "backend" > /dev/null; then
    echo "Starting backend..."
    nohup ./target/debug/backend > backend.log 2>&1 &
else
    echo "Backend is already running."
fi

if ! pgrep -x "frontend" > /dev/null; then
    echo "Starting frontend..."
    nohup ./target/debug/frontend > frontend.log 2>&1 &
else
    echo "Frontend is already running."
fi

echo "rsahp started."
