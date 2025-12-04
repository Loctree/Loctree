#!/bin/bash
set -e

echo "=== Building Loctree Landing Page ==="

cd landing
trunk build --release

echo "=== Copying to public_dist ==="
cd ..
rm -rf public_dist
cp -r landing/dist public_dist

echo ""
echo "=== Build complete! ==="
echo "Files ready in public_dist/"
echo ""
echo "Now click 'Publish' in Replit to deploy."
