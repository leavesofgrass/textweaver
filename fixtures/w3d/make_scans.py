"""Builds the scanned-page fixtures for OCR (Agent W3d).

    python fixtures/w3d/make_scans.py

Needs Pillow and the Windows fonts Times New Roman, Georgia, and Arial
(C:\\Windows\\Fonts). The text is written for these fixtures (no copyrighted
text). Each page is drawn at 200 dots per inch, then "scanned": a slight
tilt, paper noise, and a little blur, so the engines see something like a
real scan rather than a perfect rendering. The ground truth of each page is
written next to it as a .txt file (one line per printed line).

- scan-en.pdf: two pages with no text layer. Page 1 is a greyscale JPEG
  (DCTDecode); page 2 is a 1-bit image (CCITT Group 4), as office
  scanners write them.
- scan-fr.png: a French page with accented letters, as an image file.
- scan-small.png: a small English page (100 dots per inch), a hard case.
"""

import pathlib
import random

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = pathlib.Path(__file__).resolve().parent
FONTS = pathlib.Path("C:/Windows/Fonts")
DPI = 200
W, H = int(8.5 * DPI), int(11 * DPI)

EN_1 = [
    ("title", "Reading Scanned Pages Aloud"),
    ("body", "A scanned page is a picture of text. Before a computer can read it"),
    ("body", "aloud, the letters in the picture must be recognized one by one and"),
    ("body", "turned back into words. This step is called optical character"),
    ("body", "recognition, and it is never perfect."),
    ("gap", ""),
    ("head", "Why quality matters"),
    ("body", "A student who listens to a textbook hears every mistake the engine"),
    ("body", "makes. A misread number in a table or a dropped word in a sentence"),
    ("body", "can change the meaning of a paragraph, so the text must be checked"),
    ("body", "against the page whenever a detail matters."),
    ("gap", ""),
    ("head", "What helps"),
    ("body", "Clean scans at three hundred dots per inch, straight pages, and dark"),
    ("body", "print on white paper all help. Colored paper, faint photocopies, and"),
    ("body", "handwritten notes in the margins make recognition harder."),
]

EN_2 = [
    ("head", "Page two: numbers and punctuation"),
    ("body", "The survey of 1,204 students found that 87 percent preferred to"),
    ("body", "listen and read at the same time. The median session lasted 42"),
    ("body", "minutes; the longest took 3 hours and 15 minutes."),
    ("gap", ""),
    ("body", "Questions to ask: Is the source reliable? Who wrote it, and when?"),
    ("body", "Does the evidence (tables, figures, and quotations) support the claim?"),
]

FR = [
    ("title", "Lecture des pages numérisées"),
    ("body", "Une page numérisée est une image du texte. Avant qu'un ordinateur"),
    ("body", "puisse la lire à voix haute, chaque lettre doit être reconnue. Les"),
    ("body", "élèves français écrivent « déjà », « où », « été » et « Noël » avec"),
    ("body", "des accents, et une reconnaissance limitée à l'anglais les perd."),
    ("gap", ""),
    ("head", "Ce qui aide"),
    ("body", "Des numérisations nettes, des pages droites et une encre foncée sur"),
    ("body", "un papier blanc améliorent la qualité de la reconnaissance."),
]


def font(name, size_pt, dpi):
    return ImageFont.truetype(str(FONTS / name), int(size_pt * dpi / 72))


def draw_page(lines, dpi=DPI, seed=1):
    w, h = int(8.5 * dpi), int(11 * dpi)
    img = Image.new("L", (w, h), 255)
    d = ImageDraw.Draw(img)
    styles = {
        "title": font("georgiab.ttf", 20, dpi),
        "head": font("arialbd.ttf", 14, dpi),
        "body": font("times.ttf", 12, dpi),
    }
    y = int(1.0 * dpi)
    x = int(1.0 * dpi)
    truth = []
    for kind, text in lines:
        if kind == "gap":
            y += int(0.12 * dpi)
            continue
        f = styles[kind]
        d.text((x, y), text, font=f, fill=20)
        truth.append(text)
        step = {"title": 0.55, "head": 0.36, "body": 0.24}[kind]
        y += int(step * dpi)
    return img, truth


def scan(img, seed, tilt=0.35, noise=10):
    rnd = random.Random(seed)
    img = img.rotate(tilt, resample=Image.BICUBIC, fillcolor=255)
    img = img.filter(ImageFilter.GaussianBlur(0.6))
    px = img.load()
    w, h = img.size
    for _ in range(w * h // 60):
        x, y = rnd.randrange(w), rnd.randrange(h)
        v = px[x, y]
        px[x, y] = max(0, min(255, v + rnd.randint(-noise * 4, noise * 4)))
    return img


def main():
    p1, t1 = draw_page(EN_1)
    p2, t2 = draw_page(EN_2)
    s1 = scan(p1, 1)
    s2 = scan(p2, 2, tilt=-0.25).point(lambda v: 255 if v > 150 else 0, mode="1")
    # Page 1 as a JPEG, page 2 as a 1-bit (CCITT G4) image.
    jpeg = HERE / "_page1.jpg"
    s1.save(jpeg, quality=55)
    s1j = Image.open(jpeg)
    s1j.save(
        HERE / "scan-en.pdf",
        save_all=True,
        append_images=[s2],
        resolution=DPI,
    )
    jpeg.unlink()
    (HERE / "scan-en.txt").write_text("\n".join(t1 + t2) + "\n", encoding="utf-8")

    fr, tf = draw_page(FR, seed=3)
    scan(fr, 3).save(HERE / "scan-fr.png", optimize=True)
    (HERE / "scan-fr.txt").write_text("\n".join(tf) + "\n", encoding="utf-8")

    small, ts = draw_page(EN_1[:5], dpi=100)
    scan(small, 4, tilt=0.2, noise=6).save(HERE / "scan-small.png", optimize=True)
    (HERE / "scan-small.txt").write_text("\n".join(ts) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
