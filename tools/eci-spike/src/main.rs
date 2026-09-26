//! Feasibility spike: drive Code Factory's 32-bit eci.dll from Rust,
//! synthesize to a buffer, and time index marks against audio samples.
#![allow(non_snake_case, clippy::missing_safety_doc)]
use std::ffi::c_void;
use std::io::Write;

type H = *mut c_void;
type Cb = unsafe extern "system" fn(H, i32, i32, *mut c_void) -> i32;
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(m: *mut c_void, name: *const u8) -> *mut c_void;
}
struct State { samples: Vec<i16>, buf: Vec<i16>, marks: Vec<(i32, usize)> }

unsafe extern "system" fn callback(_h: H, msg: i32, lparam: i32, data: *mut c_void) -> i32 {
    let st = unsafe { &mut *(data as *mut State) };
    match msg {
        0 => { let n = lparam as usize; st.samples.extend_from_slice(&st.buf[..n]); }
        2 | 4 => st.marks.push((if msg == 4 { 100_000 + lparam } else { lparam }, st.samples.len())),
        _ => {}
    }
    1
}
macro_rules! sym { ($m:expr, $n:literal, $t:ty) => {{
    let p = unsafe { GetProcAddress($m, concat!($n, "\0").as_ptr()) };
    assert!(!p.is_null(), "missing {}", $n);
    unsafe { std::mem::transmute::<*mut c_void, $t>(p) }
}}}

fn main() {
    let dll = r"C:\Program Files (x86)\Code Factory\Eloquence for Windows\eci.dll";
    let w: Vec<u16> = dll.encode_utf16().chain([0]).collect();
    let m = unsafe { LoadLibraryW(w.as_ptr()) };
    assert!(!m.is_null(), "LoadLibrary failed");
    let eciVersion = sym!(m, "eciVersion", unsafe extern "system" fn(*mut u8));
    let eciNew = sym!(m, "eciNew", unsafe extern "system" fn() -> H);
    let eciAddText = sym!(m, "eciAddText", unsafe extern "system" fn(H, *const u8) -> i32);
    let eciInsertIndex = sym!(m, "eciInsertIndex", unsafe extern "system" fn(H, i32) -> i32);
    let eciSynthesize = sym!(m, "eciSynthesize", unsafe extern "system" fn(H) -> i32);
    let eciSynchronize = sym!(m, "eciSynchronize", unsafe extern "system" fn(H) -> i32);
    let eciRegisterCallback = sym!(m, "eciRegisterCallback", unsafe extern "system" fn(H, Cb, *mut c_void));
    let eciSetOutputBuffer = sym!(m, "eciSetOutputBuffer", unsafe extern "system" fn(H, i32, *mut i16) -> i32);
    let eciSetParam = sym!(m, "eciSetParam", unsafe extern "system" fn(H, i32, i32) -> i32);
    let eciGetParam = sym!(m, "eciGetParam", unsafe extern "system" fn(H, i32) -> i32);
    let eciSetVoiceParam = sym!(m, "eciSetVoiceParam", unsafe extern "system" fn(H, i32, i32, i32) -> i32);
    let eciDelete = sym!(m, "eciDelete", unsafe extern "system" fn(H) -> H);

    let mut v = [0u8; 64];
    unsafe { eciVersion(v.as_mut_ptr()) };
    println!("eci version: {}", String::from_utf8_lossy(&v[..v.iter().position(|&b| b == 0).unwrap_or(0)]));
    let h = unsafe { eciNew() };
    assert!(!h.is_null(), "eciNew failed");
    let mut st = Box::new(State { samples: Vec::new(), buf: vec![0; 4096], marks: Vec::new() });
    unsafe {
        eciRegisterCallback(h, callback, &mut *st as *mut State as *mut c_void);
        eciSetOutputBuffer(h, st.buf.len() as i32, st.buf.as_mut_ptr());
        eciSetParam(h, 1, 1); // eciInputType: annotations on
        eciSetParam(h, 12, 1); // eciWantWordIndex
        eciSetVoiceParam(h, 0, 6, 60); // eciSpeed on the active voice
    }
    let rate_code = unsafe { eciGetParam(h, 5) };
    let rate = [8000, 11025, 22050][rate_code.clamp(0, 2) as usize];
    let text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé.";
    // Latin-1 encoding for the Western-language engines.
    let mut words = Vec::new();
    for (i, word) in text.split(' ').enumerate() {
        unsafe { eciInsertIndex(h, i as i32) };
        let mut b: Vec<u8> = word.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect();
        b.extend_from_slice(b" \0");
        unsafe { eciAddText(h, b.as_ptr()) };
        words.push(word);
    }
    unsafe { eciInsertIndex(h, 999) };
    let t0 = std::time::Instant::now();
    unsafe { eciSynthesize(h); eciSynchronize(h); }
    let took = t0.elapsed();
    println!("sample rate {rate}, {} samples = {} ms audio, synthesized in {} ms",
        st.samples.len(), st.samples.len() * 1000 / rate, took.as_millis());
    for (idx, at) in &st.marks {
        let label = if *idx >= 100_000 { format!("word-index {}", idx - 100_000) }
            else { format!("index {idx:>3} {:?}", words.get(*idx as usize).copied().unwrap_or("<end>")) };
        println!("{:>6} ms  {label}", at * 1000 / rate);
    }
    // WAV out
    let out = std::env::args().nth(1).unwrap_or("eci.wav".into());
    let mut f = std::fs::File::create(&out).unwrap();
    let data_len = (st.samples.len() * 2) as u32;
    let mut hdr = Vec::new();
    hdr.extend_from_slice(b"RIFF"); hdr.extend_from_slice(&(36 + data_len).to_le_bytes());
    hdr.extend_from_slice(b"WAVEfmt "); hdr.extend_from_slice(&16u32.to_le_bytes());
    hdr.extend_from_slice(&1u16.to_le_bytes()); hdr.extend_from_slice(&1u16.to_le_bytes());
    hdr.extend_from_slice(&(rate as u32).to_le_bytes()); hdr.extend_from_slice(&((rate * 2) as u32).to_le_bytes());
    hdr.extend_from_slice(&2u16.to_le_bytes()); hdr.extend_from_slice(&16u16.to_le_bytes());
    hdr.extend_from_slice(b"data"); hdr.extend_from_slice(&data_len.to_le_bytes());
    f.write_all(&hdr).unwrap();
    for s in &st.samples { f.write_all(&s.to_le_bytes()).unwrap(); }
    unsafe { eciDelete(h) };
    println!("wrote {out}");
}
