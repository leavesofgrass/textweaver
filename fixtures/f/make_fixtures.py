"""Makes the file browser's fixtures (Wave 6, W6f).

Run from this folder with `py -3 make_fixtures.py` (Windows) or
`python3 make_fixtures.py`; the zips are written next to it and are
committed, so tests never need Python.

- course.zip: a course folder as a student gets it, with a macOS resource
  fork and a Finder file (junk the browser leaves out), a folder inside a
  folder, an archive inside it, and a file textweaver cannot read.
- deep.zip: archives nested six deep, past the browser's limit of four.
- broken.zip: starts like a zip and is not one.
"""

import io
import zipfile

FIXED = (2026, 9, 29, 12, 0, 0)


def zip_bytes(files):
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        for name, body in files:
            info = zipfile.ZipInfo(name, FIXED)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, body)
    return buf.getvalue()


def main():
    extra = zip_bytes(
        [
            ("reading.md", "# Extra reading\n\nThe extra reading is optional.\n"),
        ]
    )
    course = zip_bytes(
        [
            ("syllabus.md", "# Syllabus\n\nThis course meets on Tuesdays.\n"),
            (
                "week1/notes.md",
                "# Week one notes\n\nCells are the smallest units of life. "
                "They were first described in 1665.\n",
            ),
            ("week1/slides/intro.md", "# Introduction\n\nWelcome to the course.\n"),
            ("week1/data.bin", "\x00\x01\x02"),
            ("week1/extra.zip", extra),
            ("week2/notes.md", "# Week two notes\n\nTissues are groups of cells.\n"),
            ("__MACOSX/week1/._notes.md", "resource fork"),
            ("week1/.DS_Store", "finder"),
        ]
    )
    with open("course.zip", "wb") as f:
        f.write(course)

    inner = zip_bytes([("end.md", "# The end\n\nToo deep to reach.\n")])
    for i in range(6):
        inner = zip_bytes([(f"level{i}.zip", inner)])
    with open("deep.zip", "wb") as f:
        f.write(inner)

    with open("broken.zip", "wb") as f:
        f.write(b"PK\x03\x04this is not really a zip archive")


if __name__ == "__main__":
    main()
