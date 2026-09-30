//! Makes onnx-community's int8 Whisper models safe for RTen on every x86
//! CPU (feature `rten`).
//!
//! RTen's int8 matrix kernels on x86-64 without VNNI (AVX2 only, which
//! includes AMD Zen 3 and Intel before Ice Lake) use `vpmaddubsw`, which
//! adds pairs of 16-bit products and saturates when the weights use the
//! full 8-bit range. RTen's own quantization tool avoids this by keeping
//! weights to 7 bits ("range reduction", `docs/quantization.md` in RTen).
//! onnx-community's files use all 8, and on such a CPU Whisper produces
//! nonsense ("s s s s").
//!
//! [`reduce_weight_range`] fixes a model as it is loaded: every int8
//! weight that feeds a `MatMulInteger` or `ConvInteger` is halved and its
//! scale doubled, in place in the protobuf bytes (the sizes do not
//! change), so the product is the same with 7-bit weights. The loss of
//! one bit of weight precision is what RTen's own tool accepts.
//!
//! The ONNX file format is protobuf; this walks just enough of it:
//! `ModelProto.graph` (7), `GraphProto.node` (1) and `initializer` (5),
//! `NodeProto.input` (1), `op_type` (4), and `attribute` (5) for the
//! branches of an `If` (`AttributeProto.g` 6 and `graphs` 11), and
//! `TensorProto.data_type` (2), `name` (8), `float_data` (4), and
//! `raw_data` (9).

use std::collections::HashSet;
use std::ops::Range;

/// ONNX `TensorProto.DataType` values.
const FLOAT: u64 = 1;
const INT8: u64 = 3;

/// What [`reduce_weight_range`] changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Patched {
    /// Weight tensors halved.
    pub weights: usize,
    /// Scales doubled.
    pub scales: usize,
}

#[derive(Default)]
struct Found {
    /// Names of the int8 weights of integer matrix products.
    weight_names: HashSet<String>,
    /// Every initializer: name, data type, where its data is, and whether
    /// that data is packed floats (`float_data`) rather than `raw_data`.
    tensors: Vec<(String, u64, Range<usize>, bool)>,
}

fn varint(b: &[u8], pos: &mut usize) -> Option<u64> {
    let mut v = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *b.get(*pos)?;
        *pos += 1;
        v |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

/// Calls `f(field, wire_type, value range or varint)` for each field of
/// the message in `b[range]`.
fn fields(b: &[u8], range: Range<usize>, mut f: impl FnMut(u64, Range<usize>, u64)) -> Option<()> {
    let mut pos = range.start;
    while pos < range.end {
        let key = varint(b, &mut pos)?;
        let (field, wire) = (key >> 3, key & 7);
        match wire {
            0 => {
                let v = varint(b, &mut pos)?;
                f(field, pos..pos, v);
            }
            1 => pos += 8,
            2 => {
                let len = usize::try_from(varint(b, &mut pos)?).ok()?;
                let end = pos.checked_add(len)?;
                if end > range.end {
                    return None;
                }
                f(field, pos..end, 0);
                pos = end;
            }
            5 => pos += 4,
            _ => return None,
        }
    }
    (pos == range.end).then_some(())
}

fn string(b: &[u8], r: &Range<usize>) -> String {
    String::from_utf8_lossy(&b[r.clone()]).into_owned()
}

fn walk_graph(b: &[u8], graph: Range<usize>, found: &mut Found) -> Option<()> {
    let mut nodes = Vec::new();
    let mut inits = Vec::new();
    fields(b, graph, |field, r, _| match field {
        1 => nodes.push(r),
        5 => inits.push(r),
        _ => {}
    })?;
    for node in nodes {
        let mut inputs = Vec::new();
        let mut op = String::new();
        let mut attrs = Vec::new();
        fields(b, node, |field, r, _| match field {
            1 => inputs.push(string(b, &r)),
            4 => op = string(b, &r),
            5 => attrs.push(r),
            _ => {}
        })?;
        if matches!(op.as_str(), "MatMulInteger" | "ConvInteger")
            && let Some(w) = inputs.get(1)
        {
            found.weight_names.insert(w.clone());
        }
        for a in attrs {
            let mut subgraphs = Vec::new();
            fields(b, a, |field, r, _| {
                if field == 6 || field == 11 {
                    subgraphs.push(r);
                }
            })?;
            for g in subgraphs {
                walk_graph(b, g, found)?;
            }
        }
    }
    for t in inits {
        let (mut name, mut dtype, mut raw, mut floats) = (String::new(), 0, None, None);
        fields(b, t, |field, r, v| match field {
            2 => dtype = v,
            8 => name = string(b, &r),
            9 => raw = Some(r),
            4 => floats = Some(r),
            _ => {}
        })?;
        if let Some(r) = raw {
            found.tensors.push((name, dtype, r, false));
        } else if let Some(r) = floats {
            found.tensors.push((name, dtype, r, true));
        }
    }
    Some(())
}

/// The scale that belongs to a quantized weight, by onnxruntime's naming:
/// `X_quantized` has its scale in `X_scale`.
fn scale_name(weight: &str) -> Option<String> {
    weight
        .strip_suffix("_quantized")
        .map(|base| format!("{base}_scale"))
}

/// Halves every int8 weight of an integer matrix product and doubles its
/// scale, in place. A model with no such weights, or one this cannot
/// read, is left unchanged (and `None` returned for the latter). A weight
/// whose scale cannot be found is left alone.
pub fn reduce_weight_range(model: &mut [u8]) -> Option<Patched> {
    let mut graph = None;
    fields(model, 0..model.len(), |field, r, _| {
        if field == 7 {
            graph = Some(r);
        }
    })?;
    let mut found = Found::default();
    walk_graph(model, graph?, &mut found)?;
    let mut patched = Patched::default();
    let mut scales_to_double: HashSet<String> = HashSet::new();
    let mut weights_to_halve: Vec<Range<usize>> = Vec::new();
    for (name, dtype, range, packed) in &found.tensors {
        if *dtype != INT8 || *packed || !found.weight_names.contains(name) {
            continue;
        }
        let Some(scale) = scale_name(name) else {
            continue;
        };
        if !found
            .tensors
            .iter()
            .any(|(n, d, r, _)| *n == scale && *d == FLOAT && r.len() == 4)
        {
            continue;
        }
        scales_to_double.insert(scale);
        weights_to_halve.push(range.clone());
    }
    for r in weights_to_halve {
        for byte in &mut model[r] {
            let v = i16::from(*byte as i8);
            // Round half away from zero, into [-64, 64].
            let h = if v >= 0 { (v + 1) / 2 } else { (v - 1) / 2 };
            *byte = h.clamp(-64, 63) as i8 as u8;
        }
        patched.weights += 1;
    }
    for (name, _, range, _) in &found.tensors {
        if scales_to_double.contains(name) {
            let bytes: [u8; 4] = model[range.clone()].try_into().ok()?;
            let doubled = f32::from_le_bytes(bytes) * 2.0;
            model[range.clone()].copy_from_slice(&doubled.to_le_bytes());
            patched.scales += 1;
        }
    }
    Some(patched)
}

/// One field of a message: its number, wire type, where the whole entry
/// (key included) is, and its payload (a length-delimited value) or its
/// varint.
struct Entry {
    field: u64,
    wire: u64,
    whole: Range<usize>,
    payload: Range<usize>,
    value: u64,
}

/// The fields of the message in `b[range]`, with their places.
fn entries(b: &[u8], range: Range<usize>) -> Option<Vec<Entry>> {
    let mut out = Vec::new();
    let mut pos = range.start;
    while pos < range.end {
        let start = pos;
        let key = varint(b, &mut pos)?;
        let (field, wire) = (key >> 3, key & 7);
        let (payload, value) = match wire {
            0 => {
                let v = varint(b, &mut pos)?;
                (pos..pos, v)
            }
            1 => {
                pos = pos.checked_add(8)?;
                (pos - 8..pos, 0)
            }
            2 => {
                let len = usize::try_from(varint(b, &mut pos)?).ok()?;
                let p = pos;
                pos = pos.checked_add(len)?;
                (p..pos, 0)
            }
            5 => {
                pos = pos.checked_add(4)?;
                (pos - 4..pos, 0)
            }
            _ => return None,
        };
        if pos > range.end {
            return None;
        }
        out.push(Entry {
            field,
            wire,
            whole: start..pos,
            payload,
            value,
        });
    }
    Some(out)
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_bytes(out: &mut Vec<u8>, field: u64, data: &[u8]) {
    put_varint(out, field << 3 | 2);
    put_varint(out, data.len() as u64);
    out.extend_from_slice(data);
}

/// The name of Whisper's positional embedding in onnx-community's
/// encoder exports: 1500 rows (30 seconds of 20 ms frames) of the model's
/// width, added to the audio after the two convolutions.
pub const POSITIONS: &str = "embed_positions.weight";

/// A copy of a Whisper encoder that takes `frames` encoder frames (20 ms
/// each; 500 for 10 seconds) instead of 1500, by keeping the first
/// `frames` rows of its positional embedding, as whisper.cpp's
/// `audio_ctx` does. The encoder's cost then follows the window. `None`
/// when the model has no such embedding in a form this can read, or
/// `frames` is not shorter than it.
pub fn shorten_encoder(model: &[u8], frames: usize) -> Option<Vec<u8>> {
    let top = entries(model, 0..model.len())?;
    let graph = top.iter().find(|e| e.field == 7 && e.wire == 2)?;
    let inits = entries(model, graph.payload.clone())?;
    let mut target = None;
    for e in inits.iter().filter(|e| e.field == 5 && e.wire == 2) {
        let fs = entries(model, e.payload.clone())?;
        let named = fs.iter().any(|f| {
            f.field == 8 && f.wire == 2 && model[f.payload.clone()] == *POSITIONS.as_bytes()
        });
        if named {
            target = Some((e, fs));
            break;
        }
    }
    let (init, tensor) = target?;
    let mut dims: Vec<u64> = Vec::new();
    for f in tensor.iter().filter(|f| f.field == 1) {
        match f.wire {
            0 => dims.push(f.value),
            2 => {
                let mut p = f.payload.start;
                while p < f.payload.end {
                    dims.push(varint(model, &mut p)?);
                }
            }
            _ => return None,
        }
    }
    let dtype = tensor.iter().find(|f| f.field == 2 && f.wire == 0)?.value;
    let raw = tensor.iter().find(|f| f.field == 9 && f.wire == 2)?;
    let [rows, width] = dims[..] else {
        return None;
    };
    let (rows, width) = (usize::try_from(rows).ok()?, usize::try_from(width).ok()?);
    if dtype != FLOAT || frames == 0 || frames >= rows || raw.payload.len() != rows * width * 4 {
        return None;
    }
    // The tensor, with the new first dimension and the first rows.
    let mut t = Vec::new();
    let mut dims_written = false;
    for f in &tensor {
        match f.field {
            1 => {
                if !dims_written {
                    for d in [frames, width] {
                        put_varint(&mut t, 1 << 3);
                        put_varint(&mut t, d as u64);
                    }
                    dims_written = true;
                }
            }
            9 => {
                let start = raw.payload.start;
                put_bytes(&mut t, 9, &model[start..start + frames * width * 4]);
            }
            _ => t.extend_from_slice(&model[f.whole.clone()]),
        }
    }
    // The graph with that tensor in its place, then the model.
    let mut g = Vec::with_capacity(graph.payload.len());
    g.extend_from_slice(&model[graph.payload.start..init.whole.start]);
    put_bytes(&mut g, 5, &t);
    g.extend_from_slice(&model[init.whole.end..graph.payload.end]);
    let mut out = Vec::with_capacity(model.len());
    out.extend_from_slice(&model[..graph.whole.start]);
    put_bytes(&mut out, 7, &g);
    out.extend_from_slice(&model[graph.whole.end..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_positions_are_cut_to_the_window() {
        // A model: an opset field, then a graph with a node and the
        // embedding (4 rows of 2), then a trailing field.
        let floats: Vec<u8> = (0..8u8).flat_map(|i| f32::from(i).to_le_bytes()).collect();
        let mut tensor = Vec::new();
        for d in [4u64, 2] {
            tensor.extend(key(1, 0));
            tensor.extend(enc(d));
        }
        tensor.extend(key(2, 0));
        tensor.extend(enc(FLOAT));
        tensor.extend(bytes(8, POSITIONS.as_bytes()));
        tensor.extend(bytes(9, &floats));
        let mut graph = bytes(1, &node("Add", &["x", POSITIONS], None));
        graph.extend(bytes(5, &tensor));
        graph.extend(bytes(2, b"main"));
        let mut model = key(1, 0);
        model.extend(enc(8));
        model.extend(bytes(7, &graph));
        model.extend(bytes(20, b"tail"));

        let short = shorten_encoder(&model, 3).unwrap();
        let top = entries(&short, 0..short.len()).unwrap();
        assert_eq!(top.iter().map(|e| e.field).collect::<Vec<_>>(), [1, 7, 20]);
        let g = &top[1];
        let gs = entries(&short, g.payload.clone()).unwrap();
        assert_eq!(gs.iter().map(|e| e.field).collect::<Vec<_>>(), [1, 5, 2]);
        let ts = entries(&short, gs[1].payload.clone()).unwrap();
        let dims: Vec<u64> = ts
            .iter()
            .filter(|f| f.field == 1)
            .map(|f| f.value)
            .collect();
        assert_eq!(dims, [3, 2]);
        let raw = ts.iter().find(|f| f.field == 9).unwrap();
        assert_eq!(&short[raw.payload.clone()], &floats[..24]);
        // Not shorter, or no embedding: left alone.
        assert!(shorten_encoder(&model, 4).is_none());
        assert!(shorten_encoder(&model[..3], 2).is_none());
    }

    fn key(field: u64, wire: u64) -> Vec<u8> {
        enc(field << 3 | wire)
    }

    fn enc(mut v: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return out;
            }
            out.push(b | 0x80);
        }
    }

    fn bytes(field: u64, data: &[u8]) -> Vec<u8> {
        let mut out = key(field, 2);
        out.extend(enc(data.len() as u64));
        out.extend_from_slice(data);
        out
    }

    fn tensor(name: &str, dtype: u64, raw: &[u8]) -> Vec<u8> {
        let mut t = key(2, 0);
        t.extend(enc(dtype));
        t.extend(bytes(8, name.as_bytes()));
        t.extend(bytes(9, raw));
        t
    }

    fn node(op: &str, inputs: &[&str], subgraph: Option<&[u8]>) -> Vec<u8> {
        let mut n = Vec::new();
        for i in inputs {
            n.extend(bytes(1, i.as_bytes()));
        }
        n.extend(bytes(4, op.as_bytes()));
        if let Some(g) = subgraph {
            let mut attr = bytes(1, b"then_branch");
            attr.extend(bytes(6, g));
            n.extend(bytes(5, &attr));
        }
        n
    }

    #[test]
    fn halves_weights_and_doubles_scales_even_in_branches() {
        // A branch graph with its own MatMulInteger and weights.
        let mut branch = bytes(1, &node("MatMulInteger", &["a", "w2_quantized"], None));
        branch.extend(bytes(5, &tensor("w2_quantized", INT8, &[127, 0x80, 3])));
        // onnx-community's files keep scales as packed `float_data`.
        let mut packed = key(2, 0);
        packed.extend(enc(FLOAT));
        packed.extend(bytes(8, b"w2_scale"));
        packed.extend(bytes(4, &0.5f32.to_le_bytes()));
        branch.extend(bytes(5, &packed));
        let mut graph = bytes(1, &node("MatMulInteger", &["a", "w_quantized"], None));
        graph.extend(bytes(1, &node("If", &["cond"], Some(&branch))));
        graph.extend(bytes(5, &tensor("w_quantized", INT8, &[100, 0xff, 0x9c])));
        graph.extend(bytes(5, &tensor("w_scale", FLOAT, &0.25f32.to_le_bytes())));
        // Not a matrix weight: untouched.
        graph.extend(bytes(5, &tensor("other", INT8, &[100])));
        let mut model = key(1, 0);
        model.extend(enc(8));
        model.extend(bytes(7, &graph));
        let before = model.len();

        let p = reduce_weight_range(&mut model).unwrap();
        assert_eq!(
            p,
            Patched {
                weights: 2,
                scales: 2
            }
        );
        assert_eq!(model.len(), before);
        let find = |needle: &[u8]| model.windows(needle.len()).any(|w| w == needle);
        // 100, -1, -100 become 50, -1, -50; 127, -128, 3 become 63, -64, 2.
        assert!(find(&[50, 0xff, 0xce]));
        assert!(find(&[63, 0xc0, 2]));
        assert!(find(&0.5f32.to_le_bytes()));
        assert!(find(&1.0f32.to_le_bytes()));
        assert!(find(&bytes(9, &[100])));
    }

    #[test]
    fn unreadable_input_is_left_alone() {
        let mut junk = vec![0xff, 0xff, 0xff];
        assert_eq!(reduce_weight_range(&mut junk), None);
        assert_eq!(junk, vec![0xff, 0xff, 0xff]);
        assert_eq!(scale_name("x_quantized").as_deref(), Some("x_scale"));
        assert_eq!(scale_name("x"), None);
    }
}
