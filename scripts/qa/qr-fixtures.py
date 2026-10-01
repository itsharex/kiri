"""Generate public fixtures locally with qrcode 8.2 and Pillow. Never capture screens."""
from pathlib import Path
import qrcode
from PIL import Image, ImageDraw

directory = Path(__file__).resolve().parents[2] / "src-tauri/tests/fixtures/qr"
directory.mkdir(parents=True, exist_ok=True)
payloads = {"url": "https://example.org/kiri-safe", "text": "Kiri QR 测试", "unsafe": "javascript:alert(1)"}
images = {}
for name, text in payloads.items():
    code = qrcode.QRCode(error_correction=qrcode.constants.ERROR_CORRECT_M, box_size=8, border=4)
    code.add_data(text)
    code.make(fit=True)
    image = code.make_image(fill_color="black", back_color="white").convert("RGB")
    image.save(directory / f"{name}.png")
    images[name] = image

multi = Image.new("RGB", (960, 480), "white")
for name, point in [("url", (32, 32)), ("text", (352, 140)), ("url", (672, 32))]:
    multi.paste(images[name], point)
multi.save(directory / "multi.png")
damaged = images["url"].copy()
draw = ImageDraw.Draw(damaged)
size = damaged.width
draw.rectangle((size * .36, size * .36, size * .77, size * .77), fill="white")
damaged.save(directory / "damaged.png")
Image.new("RGB", (640, 360), "white").save(directory / "empty.png")
