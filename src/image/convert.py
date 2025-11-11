#!/usr/bin/env python3
"""
Converts a TGA file (any format) to 24-bit RGB TGA (no alpha),
which is compatible with the Rust `tinytga` crate.
GIMP creates RGBA!
"""

import sys
import os
from PIL import Image

def convert_tga_to_rgb24(input_path: str, output_path: str):
    if not os.path.isfile(input_path):
        raise FileNotFoundError(f"Input file not found: {input_path}")

    # Open image
    with Image.open(input_path) as img:
        print(f"Original mode: {img.mode}, size: {img.size}")

        # Convert to RGB (drops alpha if present)
        if img.mode in ("RGBA", "LA", "P"):
            # Optional: specify background color for flattening (default: white)
            background = Image.new("RGB", img.size, (255, 255, 255))
            if img.mode == "P":
                img = img.convert("RGBA")
            background.paste(img, mask=img.split()[-1] if img.mode == "RGBA" else None)
            img = background
        elif img.mode != "RGB":
            img = img.convert("RGB")

        # Save as 24-bit uncompressed TGA
        img.save(output_path, format="TGA", compression=None)
        print(f"Saved 24-bit RGB TGA to: {output_path}")

def main():
    if len(sys.argv) != 3:
        print("Usage: python convert_tga_to_rgb24.py <input.tga> <output.tga>")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]

    try:
        convert_tga_to_rgb24(input_file, output_file)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()