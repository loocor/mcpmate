#!/bin/bash

# MCPMate AI Optimized Build Script
# Based on ChatGPT's analysis for matching LM Studio performance

echo "🚀 Building MCPMate AI with maximum optimization..."

# Set Rust flags for Apple Silicon optimization
export RUSTFLAGS="-C target-cpu=native -C target-feature=+neon,+fma,+fp16 -C opt-level=3"

# Set OpenMP threads for maximum CPU utilization
export OMP_NUM_THREADS=$(sysctl -n hw.logicalcpu)

echo "📊 CPU Info:"
echo "  Logical CPUs: $(sysctl -n hw.logicalcpu)"
echo "  Physical CPUs: $(sysctl -n hw.physicalcpu)"

echo "🔧 Rust Flags: $RUSTFLAGS"
echo "🧵 OpenMP Threads: $OMP_NUM_THREADS"

# Build with release profile and all optimizations
echo "🔨 Building with maximum optimizations..."
cargo build --release --features "metal"

echo "✅ Build complete!"
echo ""
echo "🧪 Test with:"
echo "  cargo run --release -- --file test_input.txt --debug"
echo ""
echo "📈 Expected improvements:"
echo "  - 2-4x faster inference on Apple Silicon"
echo "  - Better Metal GPU utilization"
echo "  - Optimized SIMD instructions"
