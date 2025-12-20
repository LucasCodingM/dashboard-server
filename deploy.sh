#!/bin/bash

# Configuration
SERVER="lucas@server-lucas.local"
DEST="/home/lucas/app/dashboard"
BINARY_NAME="dashboard-server"

echo "--- Compiling in release mode ---"
cargo build --release

echo "--- Sending files to server ---"
# We send the binary, templates, and static folder
rsync -avz --delete target/release/$BINARY_NAME templates static $SERVER:$DEST

echo "--- Restarting the service ---"
ssh $SERVER "sudo systemctl restart dashboard"

echo "--- Done! ---"