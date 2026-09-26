"""Linux counterpart of the ECI spike: drive Voxin's 64-bit libibmeci.so with
ctypes, insert an index mark before every word, and print each mark's audio
time. Run in the dev container with the Voxin overlay:

    docker compose -f compose.yaml -f compose.voxin.yaml run --rm -T dev python3 tools/eci-spike/voxin_spike.py
"""
import ctypes
import os

lib = ctypes.CDLL(os.environ.get("TEXTWEAVER_ECI_LIBRARY", "libibmeci.so"))
H = ctypes.c_void_p
CB = ctypes.CFUNCTYPE(ctypes.c_int, H, ctypes.c_int, ctypes.c_long, ctypes.c_void_p)
lib.eciNew.restype = H
lib.eciAddText.argtypes = [H, ctypes.c_char_p]
lib.eciInsertIndex.argtypes = [H, ctypes.c_int]
lib.eciSynthesize.argtypes = [H]
lib.eciSynchronize.argtypes = [H]
lib.eciRegisterCallback.argtypes = [H, CB, ctypes.c_void_p]
lib.eciSetOutputBuffer.argtypes = [H, ctypes.c_int, ctypes.POINTER(ctypes.c_short)]
lib.eciGetParam.argtypes = [H, ctypes.c_int]
lib.eciSetParam.argtypes = [H, ctypes.c_int, ctypes.c_int]
lib.eciDelete.argtypes = [H]
lib.eciVersion.argtypes = [ctypes.c_char_p]

v = ctypes.create_string_buffer(64)
lib.eciVersion(v)
print("eci version:", v.value.decode())
h = lib.eciNew()
assert h, "eciNew failed"
buf = (ctypes.c_short * 4096)()
samples = 0
marks = []


def on_message(_h, msg, lparam, _data):
    global samples
    if msg == 0:
        samples += lparam
    elif msg == 2:
        marks.append((lparam, samples))
    return 1


cb = CB(on_message)
lib.eciRegisterCallback(h, cb, None)
lib.eciSetOutputBuffer(h, len(buf), buf)
lib.eciSetParam(h, 1, 1)
rate = [8000, 11025, 22050][max(0, min(2, lib.eciGetParam(h, 5)))]
text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
words = text.split(" ")
for i, w in enumerate(words):
    lib.eciInsertIndex(h, i)
    lib.eciAddText(h, (w + " ").encode("cp1252", "replace"))
lib.eciInsertIndex(h, 999)
lib.eciSynthesize(h)
lib.eciSynchronize(h)
print(f"sample rate {rate}, {samples} samples = {samples * 1000 // rate} ms")
for idx, at in marks:
    label = words[idx] if idx < len(words) else "<end>"
    print(f"{at * 1000 // rate:>6} ms  index {idx:>3} {label}")
lib.eciDelete(h)
