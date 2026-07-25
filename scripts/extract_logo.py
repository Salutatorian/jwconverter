from PIL import Image
from pathlib import Path

src = Path(
    r"C:\Users\JW\.cursor\projects\c-Users-JW-Desktop-projects-converter\assets\c__Users_JW_AppData_Roaming_Cursor_User_workspaceStorage_bf1116ffbc71219894d8bf780905d944_images_jwc-37c889e1-4876-41bf-ba04-83ed4bc4f285.png"
)
out_dir = Path(r"c:\Users\JW\Desktop\projects\converter\assets")
out_dir.mkdir(parents=True, exist_ok=True)
out = out_dir / "jwc-logo-transparent.png"

img = Image.open(src).convert("RGBA")
pixels = img.load()
w, h = img.size

for y in range(h):
    for x in range(w):
        r, g, b, a = pixels[x, y]
        brightness = max(r, g, b)
        if brightness < 28:
            pixels[x, y] = (r, g, b, 0)
        elif brightness < 45 and abs(r - g) < 8 and abs(g - b) < 8:
            alpha = int(255 * (brightness - 28) / 17)
            pixels[x, y] = (r, g, b, max(0, min(255, alpha)))

bbox = img.getbbox()
if bbox:
    pad = 24
    left = max(0, bbox[0] - pad)
    top = max(0, bbox[1] - pad)
    right = min(w, bbox[2] + pad)
    bottom = min(h, bbox[3] + pad)
    img = img.crop((left, top, right, bottom))

img.save(out, "PNG")
print(f"saved {out} size={img.size}")
