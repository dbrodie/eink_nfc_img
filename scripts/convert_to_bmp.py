#!/usr/bin/env python3
"""
Convert images to 8-bit indexed BMP format for NFC e-ink displays.

Supports both BWRY (4-color) and BWR (3-color) display formats.

Usage:
    python convert_to_bmp.py input.png output.bmp
    python convert_to_bmp.py input.jpg output.bmp --dither
    python convert_to_bmp.py input.png output.bmp --format bwr
    python convert_to_bmp.py test output.bmp  # Create test pattern (BWRY)
    python convert_to_bmp.py test output.bmp --format bwr  # Create test pattern (BWR)

Options:
    --dither      Use Floyd-Steinberg dithering (recommended for photos)
    --format FMT  Color format: 'bwry' (4-color, default) or 'bwr' (3-color)
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

# BWRY 4-color palette (RGB values)
PALETTE_BWRY = {
    'black':  (0, 0, 0),
    'white':  (255, 255, 255),
    'yellow': (255, 255, 0),
    'red':    (255, 0, 0),
}

# BWRY palette index mapping
PALETTE_INDICES_BWRY = {
    'black':  0,
    'white':  1,
    'yellow': 2,
    'red':    3,
}

# BWR 3-color palette (RGB values)
PALETTE_BWR = {
    'black':  (0, 0, 0),
    'white':  (255, 255, 255),
    'red':    (255, 0, 0),
}

# BWR palette index mapping
PALETTE_INDICES_BWR = {
    'black':  0,
    'white':  1,
    'red':    2,
}

def color_distance(c1, c2):
    """Calculate Euclidean distance between two RGB colors."""
    return sum((a - b) ** 2 for a, b in zip(c1, c2))

def nearest_color(rgb, palette):
    """Find the nearest palette color to the given RGB value."""
    min_dist = float('inf')
    nearest = 'white'
    for name, color in palette.items():
        dist = color_distance(rgb, color)
        if dist < min_dist:
            min_dist = dist
            nearest = name
    return nearest

def floyd_steinberg_dither(img, palette, palette_indices):
    """Apply Floyd-Steinberg dithering to convert to palette colors."""
    pixels = np.array(img, dtype=np.float32)
    height, width = pixels.shape[:2]
    output = np.zeros((height, width), dtype=np.uint8)

    for y in range(height):
        for x in range(width):
            old_pixel = pixels[y, x].copy()
            color_name = nearest_color(tuple(old_pixel.astype(int)), palette)
            new_pixel = np.array(palette[color_name], dtype=np.float32)
            output[y, x] = palette_indices[color_name]

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

def simple_quantize(img, palette, palette_indices):
    """Simple nearest-color quantization without dithering."""
    pixels = np.array(img)
    height, width = pixels.shape[:2]
    output = np.zeros((height, width), dtype=np.uint8)

    for y in range(height):
        for x in range(width):
            color_name = nearest_color(tuple(pixels[y, x]), palette)
            output[y, x] = palette_indices[color_name]

    return output

def create_indexed_bmp(color_array, output_path, format_type):
    """Create an 8-bit indexed BMP with appropriate palette."""
    palette_data = []

    if format_type == 'bwry':
        for name in ['black', 'white', 'yellow', 'red']:
            palette_data.extend(PALETTE_BWRY[name])
    else:  # bwr
        for name in ['black', 'white', 'red']:
            palette_data.extend(PALETTE_BWR[name])
        # Add one more entry to have 4 colors (helps with some BMP readers)
        palette_data.extend([0, 0, 0])

    # Fill rest of 256-color palette with black
    palette_data.extend([0] * (256 - 4) * 3)

    # Create indexed image
    img = Image.fromarray(color_array, mode='P')
    img.putpalette(palette_data)
    img.save(output_path, 'BMP')

def convert_image(input_path, output_path, use_dither=True, format_type='bwry'):
    """Convert an image to 8-bit indexed BMP format."""
    img = Image.open(input_path)

    if img.mode != 'RGB':
        img = img.convert('RGB')

    if img.size != (WIDTH, HEIGHT):
        img = img.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)

    # Select palette based on format
    if format_type == 'bwry':
        palette = PALETTE_BWRY
        palette_indices = PALETTE_INDICES_BWRY
        color_count = 4
    else:  # bwr
        palette = PALETTE_BWR
        palette_indices = PALETTE_INDICES_BWR
        color_count = 3

    if use_dither:
        print(f"Applying Floyd-Steinberg dithering ({color_count}-color {format_type.upper()})...")
        color_array = floyd_steinberg_dither(img, palette, palette_indices)
    else:
        print(f"Applying simple quantization ({color_count}-color {format_type.upper()})...")
        color_array = simple_quantize(img, palette, palette_indices)

    create_indexed_bmp(color_array, output_path, format_type)
    print(f"Converted {input_path} -> {output_path}")

def create_test_pattern(output_path, format_type='bwry'):
    """Create a test pattern image with colored stripes."""
    color_array = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)

    if format_type == 'bwry':
        # 4 horizontal stripes: black, white, yellow, red
        num_colors = 4
        stripe_height = HEIGHT // num_colors
        print(f"Creating BWRY test pattern (4 colors)...")
    else:  # bwr
        # 3 horizontal stripes: black, white, red
        num_colors = 3
        stripe_height = HEIGHT // num_colors
        print(f"Creating BWR test pattern (3 colors)...")

    for y in range(HEIGHT):
        color = min(y // stripe_height, num_colors - 1)
        for x in range(WIDTH):
            color_array[y, x] = color

    create_indexed_bmp(color_array, output_path, format_type)
    print(f"Created test pattern: {output_path}")

def parse_args():
    """Parse command line arguments."""
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    input_arg = sys.argv[1]
    output_path = sys.argv[2]
    use_dither = '--dither' in sys.argv

    # Parse --format argument
    format_type = 'bwry'  # default
    for i, arg in enumerate(sys.argv):
        if arg == '--format' and i + 1 < len(sys.argv):
            format_type = sys.argv[i + 1].lower()
            if format_type not in ('bwr', 'bwry'):
                print(f"Error: Invalid format '{format_type}'. Use 'bwr' or 'bwry'.")
                sys.exit(1)
            break

    return input_arg, output_path, use_dither, format_type

def main():
    input_arg, output_path, use_dither, format_type = parse_args()

    if input_arg == 'test':
        create_test_pattern(output_path, format_type)
    else:
        convert_image(input_arg, output_path, use_dither, format_type)

if __name__ == '__main__':
    main()
