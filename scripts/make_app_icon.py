"""Create a square app icon from the transparent JWC logo."""
from PIL import Image
from pathlib import Path

logo_path = Path(r"c:\Users\JW\Desktop\projects\converter\assets\jwc-logo-transparent.png")
out_path = Path(r"c:\Users\JW\Desktop\projects\converter\assets\jwc-icon-1024.png")

logo = Image.open(logo_path).convert("RGBA")
size = 1024
canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))

# Fit logo width with padding
max_w = int(size * 0.86)
max_h = int(size * 0.5)
scale = min(max_w / logo.width, max_h / logo.height)
new_size = (max(1, int(logo.width * scale)), max(1, int(logo.height * scale)))
logo = logo.resize(new_size, Image.Resampling.LANCZOS)

x = (size - logo.width) // 2
y = (size - logo.height) // 2
canvas.paste(logo, (x, y), logo)
canvas.save(out_path, "PNG")
print(f"wrote {out_path}")
