#!/bin/bash
set -e

echo "Building rsahp..."
cargo build --bin backend
cargo build --bin frontend

mkdir -p logs

if pgrep -x "backend" > /dev/null; then
    echo "Backend is already running."
else
    echo "Starting backend..."
    ./target/debug/backend > logs/backend_out.log 2> logs/backend_err.log &
    echo $! > logs/backend.pid
fi

if pgrep -x "frontend" > /dev/null; then
    echo "Frontend is already running."
else
    echo "Starting frontend..."
    ./target/debug/frontend > logs/frontend_out.log 2> logs/frontend_err.log &
    echo $! > logs/frontend.pid
fi

echo "rsahp started."
