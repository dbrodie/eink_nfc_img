#!/usr/bin/env python3
"""
Convert images to .4ei format for DMPL0154FN1 e-ink display.

Usage:
    python convert_to_4ei.py input.png output.4ei
    python convert_to_4ei.py input.jpg output.4ei --dither
"""

import sys
import struct
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

# 2-bit color codes
COLOR_CODES = {
    'black':  0b00,
    'white':  0b01,
    'yellow': 0b10,
    'red':    0b11,
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
    # Convert to numpy array for faster processing
    pixels = np.array(img, dtype=np.float32)
    height, width = pixels.shape[:2]

    # Output array
    output = np.zeros((height, width), dtype=np.uint8)

    for y in range(height):
        for x in range(width):
            old_pixel = pixels[y, x].copy()

            # Find nearest color
            color_name = nearest_color(tuple(old_pixel.astype(int)))
            new_pixel = np.array(PALETTE[color_name], dtype=np.float32)
            output[y, x] = COLOR_CODES[color_name]

            # Calculate error
            error = old_pixel - new_pixel

            # Distribute error to neighboring pixels (Floyd-Steinberg coefficients)
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
            output[y, x] = COLOR_CODES[color_name]

    return output

def pack_pixels(color_array):
    """Pack 2-bit pixels into bytes (4 pixels per byte, MSB first)."""
    height, width = color_array.shape
    bytes_per_row = width // 4

    data = bytearray(height * bytes_per_row)

    for y in range(height):
        for x_byte in range(bytes_per_row):
            byte_val = 0
            for bit in range(4):
                x = x_byte * 4 + bit
                byte_val = (byte_val << 2) | (color_array[y, x] & 0x03)
            data[y * bytes_per_row + x_byte] = byte_val

    return bytes(data)

def convert_image(input_path, output_path, use_dither=True):
    """Convert an image to .4ei format."""
    # Load image
    img = Image.open(input_path)

    # Convert to RGB if necessary
    if img.mode != 'RGB':
        img = img.convert('RGB')

    # Resize to display dimensions
    if img.size != (WIDTH, HEIGHT):
        img = img.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)

    # Quantize to 4-color palette
    if use_dither:
        print("Applying Floyd-Steinberg dithering...")
        color_array = floyd_steinberg_dither(img)
    else:
        print("Applying simple quantization...")
        color_array = simple_quantize(img)

    # Pack pixels into bytes
    packed_data = pack_pixels(color_array)

    # Write .4ei file
    with open(output_path, 'wb') as f:
        # Header
        f.write(b'4EI1')  # Magic
        f.write(struct.pack('<H', WIDTH))   # Width (little-endian)
        f.write(struct.pack('<H', HEIGHT))  # Height (little-endian)
        # Image data
        f.write(packed_data)

    print(f"Converted {input_path} -> {output_path}")
    print(f"Output size: {8 + len(packed_data)} bytes")

def create_test_pattern(output_path):
    """Create a test pattern image with colored stripes."""
    color_array = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)

    # Horizontal stripes
    for y in range(HEIGHT):
        color = y // 50  # 4 stripes of 50 pixels each
        for x in range(WIDTH):
            color_array[y, x] = color % 4

    packed_data = pack_pixels(color_array)

    with open(output_path, 'wb') as f:
        f.write(b'4EI1')
        f.write(struct.pack('<H', WIDTH))
        f.write(struct.pack('<H', HEIGHT))
        f.write(packed_data)

    print(f"Created test pattern: {output_path}")

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        print("\nCommands:")
        print("  convert <input> <output.4ei> [--dither]")
        print("  test <output.4ei>  - Create a test pattern")
        sys.exit(1)

    command = sys.argv[1]

    if command == 'test':
        if len(sys.argv) < 3:
            print("Usage: python convert_to_4ei.py test output.4ei")
            sys.exit(1)
        create_test_pattern(sys.argv[2])

    elif command == 'convert' or command not in ['test']:
        # Handle both "convert input output" and "input output" forms
        if command == 'convert':
            if len(sys.argv) < 4:
                print("Usage: python convert_to_4ei.py convert input.png output.4ei [--dither]")
                sys.exit(1)
            input_path = sys.argv[2]
            output_path = sys.argv[3]
            use_dither = '--dither' in sys.argv
        else:
            if len(sys.argv) < 3:
                print("Usage: python convert_to_4ei.py input.png output.4ei [--dither]")
                sys.exit(1)
            input_path = sys.argv[1]
            output_path = sys.argv[2]
            use_dither = '--dither' in sys.argv

        convert_image(input_path, output_path, use_dither)

    else:
        print(f"Unknown command: {command}")
        sys.exit(1)

if __name__ == '__main__':
    main()
