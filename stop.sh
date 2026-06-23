#!/bin/bash

if pgrep -x "backend" > /dev/null; then
    echo "Stopping backend gracefully..."
    # SIGTERM will be caught by graceful shutdown handler
    pkill -TERM -x "backend"
else
    echo "Backend is not running."
fi

if pgrep -x "frontend" > /dev/null; then
    echo "Stopping frontend..."
    pkill -TERM -x "frontend"
else
    echo "Frontend is not running."
fi

echo "rsahp stopped."
