# File browser fixtures

These files let you try the file browser without your own documents. Open this folder from File, Browse files, then enter course.zip to walk through an archive.

- course.zip holds two weeks of notes, a folder inside a folder, an archive inside the archive, and a file textweaver cannot read.
- deep.zip nests archives six deep; the browser stops at four and says so.
- broken.zip looks like a zip and is not one; the browser says it cannot read it.

make_fixtures.py writes the three zips again.
