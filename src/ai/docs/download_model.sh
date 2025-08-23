#!/bin/bash

# MCPMate AI - Model Download Script
# Download pre-converted GGUF models for direct use

set -e

MODEL_DIR="./models"
mkdir -p "$MODEL_DIR"

echo "🤖 MCPMate AI Model Downloader"
echo "================================"

# Function to download with progress
download_with_progress() {
    local url="$1"
    local output="$2"
    local description="$3"
    
    echo "📥 Downloading $description..."
    curl -L --progress-bar -o "$output" "$url"
    echo "✅ Downloaded: $output"
}

# Option 1: Qwen2.5-0.5B (Smallest, fastest)
download_qwen25_0_5b() {
    echo "🎯 Downloading Qwen2.5-0.5B-Instruct (Q4_K_M, ~350MB)"
    download_with_progress \
        "https://huggingface.co/bartowski/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf" \
        "$MODEL_DIR/qwen2.5-0.5b-instruct-q4.gguf" \
        "Qwen2.5-0.5B-Instruct (Q4_K_M)"
}

# Option 2: Qwen2.5-1.5B (Balanced performance)
download_qwen25_1_5b() {
    echo "🎯 Downloading Qwen2.5-1.5B-Instruct (Q4_K_M, ~900MB)"
    download_with_progress \
        "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf" \
        "$MODEL_DIR/qwen2.5-1.5b-instruct-q4.gguf" \
        "Qwen2.5-1.5B-Instruct (Q4_K_M)"
}

echo "请选择要下载的模型:"
echo "1) Qwen2.5-0.5B (最小，最快，~350MB)"
echo "2) Qwen2.5-1.5B (平衡性能，~900MB)" 
echo "3) 全部下载"

read -p "选择 (1-4): " choice

case $choice in
    1)
        download_qwen25_0_5b
        DOWNLOADED_MODEL="$MODEL_DIR/qwen2.5-0.5b-instruct-q4.gguf"
        ;;
    2)
        download_qwen25_1_5b
        DOWNLOADED_MODEL="$MODEL_DIR/qwen2.5-1.5b-instruct-q4.gguf"
        ;;
    3)
        download_qwen25_0_5b
        download_qwen25_1_5b
        DOWNLOADED_MODEL="$MODEL_DIR/qwen2.5-0.5b-instruct-q4.gguf"
        ;;
    *)
        echo "❌ 无效选择"
        exit 1
        ;;
esac

echo ""
echo "🎉 下载完成!"
echo "📁 模型文件位置: $MODEL_DIR/"
echo ""
echo "🧪 测试模型:"
echo "cargo run --example basic_extraction $DOWNLOADED_MODEL"
echo ""
echo "📖 使用说明: ./USAGE.md"
