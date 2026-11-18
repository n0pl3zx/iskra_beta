"""Генерирует RGBA-лист эмодзи из системного шрифта Segoe UI Emoji.

Порядок должен совпадать с массивом EMOJIS в src/app.rs.
"""
import os

from PIL import Image, ImageDraw, ImageFont

CELL = 72
COLS, ROWS = 8, 6
FONT_SIZE = 60

EMOJIS = [
    "😀", "😄", "😁", "😊", "🙂", "😉", "😍", "🤔", "😎", "😢", "😭", "😡", "🤯", "👍",
    "👎", "👏", "🙏", "💪", "🤝", "🔥", "💯", "❤️", "💔", "🎉", "🎁", "☕", "🍕", "🌙",
    "☀️", "⚡", "⭐", "🚀", "✨", "🤗", "😅", "😴", "🤣", "😇", "🤩", "🥳", "😱", "🥺",
    "😬", "🙃", "🧠", "💎", "🎧", "🍺",
]

font = ImageFont.truetype("C:/Windows/Fonts/seguiemj.ttf", FONT_SIZE)
sheet = Image.new("RGBA", (CELL * COLS, CELL * ROWS), (0, 0, 0, 0))
draw = ImageDraw.Draw(sheet)

for i, emoji in enumerate(EMOJIS):
    col, row = i % COLS, i // COLS
    x, y = col * CELL, row * CELL
    draw.text(
        (x + CELL / 2, y + CELL / 2),
        emoji,
        font=font,
        anchor="mm",
        embedded_color=True,
    )

out = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets")
os.makedirs(out, exist_ok=True)
sheet.tobytes()
with open(os.path.join(out, "emoji_sheet.rgba"), "wb") as f:
    f.write(sheet.tobytes())
sheet.save(os.path.join(out, "emoji_sheet_preview.png"))
print(f"ok: {len(EMOJIS)} emoji, {sheet.width}x{sheet.height}")
