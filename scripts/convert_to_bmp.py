#!/usr/bin/env python3
"""
Convert images to 8-bit indexed BMP format for DMPL0154FN1 e-ink display.

Usage:
    python convert_to_bmp.py input.png output.bmp
    python convert_to_bmp.py input.jpg output.bmp --dither
    python convert_to_bmp.py test output.bmp  # Create test pattern
"""

import sys
from pathlib import Path

try:
    from PIL import Image
    import numpy as np
except ImportError:
    print("Please install required packages: pip install pillow numpy")
    sys.exit(1)

# Display dimensions
WIDTH = 200
HEIGHT = 200

# 4-color palette (RGB values)
PALETTE = {
    'black':  (0, 0, 0),
    'white':  (255, 255, 255),
    'yellow': (255, 255, 0),
    'red':    (255, 0, 0),
}

# Palette index mapping (for BMP palette order)
PALETTE_INDICES = {
    'black':  0,
    'white':  1,
    'yellow': 2,
    'red':    3,
}

def color_distance(c1, c2):
    """Calculate Euclidean distance between two RGB colors."""
    return sum((a - b) ** 2 for a, b in zip(c1, c2))

def nearest_color(rgb):
    """Find the nearest palette color to the given RGB value."""
    min_dist = float('inf')
    nearest = 'white'
    for name, color in PALETTE.items():
        dist = color_distance(rgb, color)
        if dist < min_dist:
            min_dist = dist
            nearest = name
    return nearest

def floyd_steinberg_dither(img):
    """Apply Floyd-Steinberg dithering to convert to 4-color palette."""
    pixels = np.array(img, dtype=np.float32)
    height, width = pixels.shape[:2]
    output = np.zeros((height, width), dtype=np.uint8)

    for y in range(height):
        for x in range(width):
            old_pixel = pixels[y, x].copy()
            color_name = nearest_color(tuple(old_pixel.astype(int)))
            new_pixel = np.array(PALETTE[color_name], dtype=np.float32)
            output[y, x] = PALETTE_INDICES[color_name]

            error = old_pixel - new_pixel
            if x + 1 < width:
                pixels[y, x + 1] += error * 7 / 16
            if y + 1 < height:
                if x > 0:
                    pixels[y + 1, x - 1] += error * 3 / 16
                pixels[y + 1, x] += error * 5 / 16
                if x + 1 < width:
                    pixels[y + 1, x + 1] += error * 1 / 16

    return output

def simple_quantize(img):
    """Simple nearest-color quantization without dithering."""
    pixels = np.array(img)
    height, width = pixels.shape[:2]
    output = np.zeros((height, width), dtype=np.uint8)

    for y in range(height):
        for x in range(width):
            color_name = nearest_color(tuple(pixels[y, x]))
            output[y, x] = PALETTE_INDICES[color_name]

    return output

def create_indexed_bmp(color_array, output_path):
    """Create an 8-bit indexed BMP with 4-color palette."""
    # Create palette image
    palette_data = []
    for name in ['black', 'white', 'yellow', 'red']:
        palette_data.extend(PALETTE[name])
    # Fill rest of 256-color palette with black
    palette_data.extend([0] * (256 - 4) * 3)

    # Create indexed image
    img = Image.fromarray(color_array, mode='P')
    img.putpalette(palette_data)
    img.save(output_path, 'BMP')

def convert_image(input_path, output_path, use_dither=True):
    """Convert an image to 8-bit indexed BMP format."""
    img = Image.open(input_path)

    if img.mode != 'RGB':
        img = img.convert('RGB')

    if img.size != (WIDTH, HEIGHT):
        img = img.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)

    if use_dither:
        print("Applying Floyd-Steinberg dithering...")
        color_array = floyd_steinberg_dither(img)
    else:
        print("Applying simple quantization...")
        color_array = simple_quantize(img)

    create_indexed_bmp(color_array, output_path)
    print(f"Converted {input_path} -> {output_path}")

def create_test_pattern(output_path):
    """Create a test pattern image with colored stripes."""
    color_array = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)

    # Horizontal stripes
    for y in range(HEIGHT):
        color = y // 50  # 4 stripes of 50 pixels each
        for x in range(WIDTH):
            color_array[y, x] = color % 4

    create_indexed_bmp(color_array, output_path)
    print(f"Created test pattern: {output_path}")

def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    input_arg = sys.argv[1]
    output_path = sys.argv[2]
    use_dither = '--dither' in sys.argv

    if input_arg == 'test':
        create_test_pattern(output_path)
    else:
        convert_image(input_arg, output_path, use_dither)

if __name__ == '__main__':
    main()
