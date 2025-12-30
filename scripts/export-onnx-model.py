#!/usr/bin/env python3
"""Export the ONNX model for bundling with the app.

This script requires transformers and optimum, which are NOT runtime dependencies.
Run with: uv run --with transformers --with 'optimum[onnxruntime]' scripts/export-onnx-model.py

The exported model will be placed in src-tauri/binaries/onnx-model/
"""

import sys
from pathlib import Path

# Model optimized for semantic search - trained on 215M Q&A pairs
MODEL_NAME = "multi-qa-mpnet-base-cos-v1"


def main():
    # Determine output directory
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    output_dir = project_root / "src-tauri" / "binaries" / "onnx-model"

    print(f"Exporting ONNX model: sentence-transformers/{MODEL_NAME}")
    print(f"Output directory: {output_dir}")

    # Import heavy dependencies (only needed for export)
    try:
        from optimum.onnxruntime import ORTModelForFeatureExtraction
        from transformers import AutoTokenizer
    except ImportError:
        print("\nError: Required dependencies not found.")
        print("Run with:")
        print("  uv run --with transformers --with 'optimum[onnxruntime]' scripts/export-onnx-model.py")
        sys.exit(1)

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load and export model
    hf_model = f"sentence-transformers/{MODEL_NAME}"
    print(f"\nDownloading model from HuggingFace: {hf_model}")

    tokenizer = AutoTokenizer.from_pretrained(hf_model)
    print("Tokenizer loaded")

    model = ORTModelForFeatureExtraction.from_pretrained(hf_model, export=True)
    print("Model loaded and converted to ONNX")

    # Save to output directory
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)
    print(f"\nModel exported to: {output_dir}")

    # Verify required files exist
    required_files = ["model.onnx", "tokenizer.json"]
    for fname in required_files:
        fpath = output_dir / fname
        if fpath.exists():
            size_mb = fpath.stat().st_size / (1024 * 1024)
            print(f"  ✓ {fname} ({size_mb:.1f} MB)")
        else:
            print(f"  ✗ {fname} - MISSING!")
            sys.exit(1)

    print("\nDone! Model ready for bundling.")


if __name__ == "__main__":
    main()
