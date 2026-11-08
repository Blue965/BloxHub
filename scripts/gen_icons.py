#!/usr/bin/env python3
"""Génère les icônes de BloxHub : PNG (32/128/256/512/1024), ICO multi-tailles, ICNS."""
from PIL import Image, ImageDraw, ImageFont
import struct, os

OUT = "/home/z/my-project/bloxhub/src-tauri/icons"
os.makedirs(OUT, exist_ok=True)
S = 1024

# Fond sombre arrondi + tuile rouge inclinée + "B" blanc (inspiré Roblox, pas copié)
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

def rounded(draw, xy, r, fill):
    draw.rounded_rectangle(xy, radius=r, fill=fill)

# Fond : carré sombre aux coins arrondis (marge 8px pour antialias propre)
margin = 8
rounded(d, (margin, margin, S - margin, S - margin), 180, (13, 14, 17, 255))

# Tuile rouge inclinée
tile = Image.new("RGBA", (S, S), (0, 0, 0, 0))
td = ImageDraw.Draw(tile)
rounded(td, (262, 262, 762, 762), 120, (226, 35, 26, 255))
tile = tile.rotate(-10, resample=Image.BICUBIC, center=(512, 512))
img = Image.alpha_composite(img, tile)
d = ImageDraw.Draw(img)

# Lettre B
font_path = None
for cand in [
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
]:
    if os.path.exists(cand):
        font_path = cand
        break
if font_path is None:
    raise SystemExit("Aucune police Bold trouvée")

font = ImageFont.truetype(font_path, 420)
bbox = d.textbbox((0, 0), "B", font=font)
w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
d.text((512 - w / 2 - bbox[0], 512 - h / 2 - bbox[1]), "B", font=font, fill=(255, 255, 255, 255))

# Exports PNG
img.save(f"{OUT}/icon.png")  # 1024
for size, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png"), (512, "512x512.png")]:
    img.resize((size, size), Image.LANCZOS).save(f"{OUT}/{name}")

# ICO multi-tailles
img.save(f"{OUT}/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

# ICNS manuel : chunks PNG (ic07=128, ic08=256, ic09=512, ic10=1024)
def icns_chunk(code, data):
    return code + struct.pack(">I", len(data) + 8) + data

png128 = img.resize((128, 128), Image.LANCZOS)
buf = b"icns" + b"\x00" * 4  # header provisoire, taille corrigée après
body = b""
for code, size in [(b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)]:
    import io
    b = io.BytesIO()
    img.resize((size, size), Image.LANCZOS).save(b, "PNG")
    body += icns_chunk(code, b.getvalue())
icns = b"icns" + struct.pack(">I", 8 + len(body)) + body
with open(f"{OUT}/icon.icns", "wb") as f:
    f.write(icns)

print("Icônes générées dans", OUT)
for f in sorted(os.listdir(OUT)):
    print(" -", f, os.path.getsize(os.path.join(OUT, f)), "octets")
