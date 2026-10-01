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

# Global threshold regression: a readable local contrast range surrounded by white.
contrast = Image.new("L", (960, 480), 255)
contrast.paste(images["url"].convert("L").point(lambda value: value * 100 // 255), (32, 32))
contrast.save(directory / "contrast.png")

# Public synthetic WeChat-shaped payloads; these are not live account tokens.
wechat_payloads = [
    "https://weixin.qq.com/r/KIRI_PUBLIC_FIXTURE",
    "https://login.weixin.qq.com/l/KIRI_PUBLIC_FIXTURE==",
    "https://mp.weixin.qq.com/s/KIRI_PUBLIC_FIXTURE",
    "weixin://wxpay/bizpayurl?pr=KIRI_PUBLIC_FIXTURE",
]
wechat = Image.new("RGB", (1280, 360), "white")
for index, payload in enumerate(wechat_payloads):
    code = qrcode.QRCode(error_correction=qrcode.constants.ERROR_CORRECT_H, box_size=6, border=4)
    code.add_data(payload)
    code.make(fit=True)
    image = code.make_image(fill_color="black", back_color="white").convert("RGB")
    # A small center mark simulates ordinary branded QR codes without using
    # private WeChat images or mistaking circular Mini Program codes for QR.
    side = image.width // 10
    offset = (image.width - side) // 2
    image.paste("white", (offset, offset, offset + side, offset + side))
    wechat.paste(image, (index * 320 + 8, 8))
wechat.save(directory / "wechat.png")

# Finder patterns from neighboring thumbnails can be grouped into a false grid.
# Keep one genuine, readable code beside each malformed perspective candidate.
for name, placements in {
    "cross-finders": [(542, 261, 191, False), (1149, 184, 150, True), (32, 500, 190, False)],
    "projective-pole": [(428, 225, 198, False), (650, 341, 156, False), (1094, 150, 88, True)],
}.items():
    montage = Image.new("L", (1300, 750), 255)
    for x, y, size, damaged_corner in placements:
        code = images["url"].convert("L").resize((size, size), Image.Resampling.NEAREST)
        if damaged_corner:
            start = size * 5 // 8
            extent = size * 3 // 8
            code.paste(255, (start, 0, start + extent, extent))
        montage.paste(code, (x, y))
    montage.save(directory / f"{name}.png")
