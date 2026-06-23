#!/usr/bin/env bash

echo "Stopping rsahp..."

if pgrep -x "backend" > /dev/null; then
    echo "Stopping backend..."
    pkill -x "backend"
else
    echo "Backend is not running."
fi

if pgrep -x "frontend" > /dev/null; then
    echo "Stopping frontend..."
    pkill -x "frontend"
else
    echo "Frontend is not running."
fi

echo "rsahp stopped."
