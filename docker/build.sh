#!/bin/bash

# Step 0: Build frontend with pnpm in ../web
echo "Step 0: Building frontend in ../web using pnpm..."
cd ../web || { echo "Failed to enter ../web directory"; exit 1; }
pnpm run build || { echo "Frontend build failed"; exit 1; }

# Step 1: Build Rust backend with cargo in project root
echo "Step 1: Building Rust backend with cargo..."
cd ../ || { echo "Failed to enter project root directory"; exit 1; }
cargo build --release || { echo "Rust backend build failed"; exit 1; }

# Step 2: Back to docker directory
echo "Step 2: Returning to docker directory..."
cd docker || { echo "Failed to enter docker directory"; exit 1; }

# Step 3: Extract version from Cargo.toml
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' ../Cargo.toml)
echo "Step 3: Extracted version: $VERSION"

# Step 4: Copy the compiled RustMailer binary to current directory (docker/)
cp ../target/release/bichon-server .
echo "Step 4: Copied bichon-server binary"

# Step 5: Build Docker image with version tag
sudo docker build --build-arg CRATE_VERSION=$VERSION -t bichon:$VERSION .
echo "Step 5: Built Docker image with tag bichon:$VERSION"

# Step 6: Tag local image for Docker Hub
docker tag bichon:$VERSION billydong/bichon:$VERSION
echo "Step 6: Tagged image as billydong/bichon:$VERSION"
