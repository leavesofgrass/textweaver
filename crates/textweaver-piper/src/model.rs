//! A Piper voice model run by RTen, the pure-Rust ONNX runtime.
//!
//! Based on `rten-examples/src/piper.rs`. The model takes phoneme ids
//! (`input`, `input_lengths`), the noise and length scales (`scales`), and
//! for multi-speaker voices a speaker id (`sid`); it returns audio as
//! floats (`output`, shape `[batch, 1, 1, samples]`).
//!
//! **Word timing.** The duration predictor's rounded output, `w_ceil` in
//! Piper's VITS code, holds how many audio frames each phoneme id lasts.
//! In the exported graph it is the output of the only `Ceil` node,
//! `/Ceil_output_0`, shape `[1, 1, ids]`. It is asked for as a second
//! output of the same run, so it costs nothing. A frame is 256 samples in
//! Piper's voices; the backend divides the audio's length by the total
//! frames instead of assuming it. A graph without the node (an export
//! that renamed it) still speaks, with estimated word timing.

use std::path::Path;

use rten::{Model, NodeId};
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::PiperError;
use crate::config::VoiceConfig;

/// The graph output holding each phoneme id's duration in frames.
pub const W_CEIL_NODE: &str = "/Ceil_output_0";

/// A loaded voice model.
pub struct PiperModel {
    model: Model,
    input: NodeId,
    input_lengths: NodeId,
    scales: NodeId,
    sid: Option<NodeId>,
    output: NodeId,
    w_ceil: Option<NodeId>,
}

impl std::fmt::Debug for PiperModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiperModel")
            .field("speakers", &self.sid.is_some())
            .field("word_timing", &self.w_ceil.is_some())
            .finish_non_exhaustive()
    }
}

/// What one run of the model produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inference {
    /// Audio samples, roughly in `-1.0..=1.0`.
    pub audio: Vec<f32>,
    /// Frames per input id (`w_ceil`), when the graph gives them.
    pub durations: Option<Vec<f32>>,
}

impl PiperModel {
    /// Loads the model at `onnx`. The file is read into memory (RTen's
    /// memory-mapped loading is `unsafe`, and not used).
    pub fn load(onnx: &Path) -> Result<Self, PiperError> {
        let bytes = std::fs::read(onnx).map_err(|e| PiperError::io(onnx, e))?;
        let model = Model::load(bytes)
            .map_err(|e| PiperError::Model(format!("{}: {e}", onnx.display())))?;
        let node = |name: &str| {
            model
                .find_node(name)
                .ok_or_else(|| PiperError::Model(format!("the voice model has no {name:?} node")))
        };
        let input = node("input")?;
        let input_lengths = node("input_lengths")?;
        let scales = node("scales")?;
        let output = node("output")?;
        let sid = model.find_node("sid");
        let w_ceil = model.find_node(W_CEIL_NODE);
        if w_ceil.is_none() {
            log::info!(
                "piper: {} has no {W_CEIL_NODE}; word timing will be estimated",
                onnx.display()
            );
        }
        Ok(PiperModel {
            model,
            input,
            input_lengths,
            scales,
            sid,
            output,
            w_ceil,
        })
    }

    /// True when the graph reports phoneme durations (word timing from
    /// the model).
    pub fn has_word_timing(&self) -> bool {
        self.w_ceil.is_some()
    }

    /// Runs the model on `ids` with the voice's noise scales and
    /// `length_scale` (larger is slower), as speaker `speaker` in a
    /// multi-speaker model.
    pub fn infer(
        &self,
        config: &VoiceConfig,
        ids: &[i64],
        length_scale: f32,
        speaker: u32,
    ) -> Result<Inference, PiperError> {
        if ids.is_empty() {
            return Ok(Inference::default());
        }
        // RTen keeps 64-bit integer tensors as 32-bit.
        let ids: Vec<i32> = ids.iter().map(|&i| i32::try_from(i).unwrap_or(0)).collect();
        let len = i32::try_from(ids.len())
            .map_err(|_| PiperError::Model("the text is too long to speak in one piece".into()))?;
        let input = NdTensor::from_data([1, ids.len()], ids);
        let lengths = NdTensor::from([len]);
        let scales = NdTensor::from([
            config.inference.noise_scale,
            length_scale,
            config.inference.noise_w,
        ]);
        let mut inputs: Vec<(NodeId, rten::ValueOrView<'_>)> = vec![
            (self.input, input.into()),
            (self.input_lengths, lengths.into()),
            (self.scales, scales.into()),
        ];
        if let Some(sid) = self.sid {
            let speaker = i32::try_from(speaker).unwrap_or(0);
            inputs.push((sid, NdTensor::from([speaker]).into()));
        }
        let mut outputs = vec![self.output];
        if let Some(w) = self.w_ceil {
            outputs.push(w);
        }
        let mut values = self
            .model
            .run(inputs, &outputs, None)
            .map_err(|e| PiperError::Model(format!("synthesis failed: {e}")))?
            .into_iter();
        let audio: Tensor<f32> = values
            .next()
            .ok_or_else(|| PiperError::Model("the model returned no audio".into()))?
            .try_into()
            .map_err(|e| PiperError::Model(format!("unexpected audio output: {e}")))?;
        let durations = match values.next() {
            Some(v) => {
                let d: Tensor<f32> = v
                    .try_into()
                    .map_err(|e| PiperError::Model(format!("unexpected durations: {e}")))?;
                Some(d.to_vec())
            }
            None => None,
        };
        Ok(Inference {
            audio: audio.to_vec(),
            durations,
        })
    }
}
