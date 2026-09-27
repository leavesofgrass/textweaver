//! `tw dictate`: transcribe speech with Whisper. Owner: Agent J (wave 2).
//!
//! Wave 3 (Agent W3f, ADR-0023): Whisper runs in-process on RTen when an
//! onnx-community model is in `<data>/whisper/rten/<model>` (`base.en` by
//! default) or `--engine rten` names it, and without `--file` it records
//! from the microphone until Enter. The Whisper programs are the fallback.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use serde::Serialize;
use textweaver_app::store::Paths;
use textweaver_dictation::rten_whisper::RtenWhisperFiles;
use textweaver_dictation::{
    Dictation, DictationEvent, DictationInput, MicCapture, RtenConfig, RtenDictation, Transcript,
    WHISPER_MODELS, WhisperConfig, WhisperDictation, WhisperEngine, WhisperModel,
    apply_spoken_commands, detect,
};

/// Arguments for `tw dictate`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Transcribe this audio file instead of the microphone.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Whisper model name: tiny, base, small, medium, large-v3, or
    /// large-v3-turbo.
    #[arg(long)]
    pub model: Option<String>,
    /// Which Whisper: rten (in-process, onnx-community models), cpp
    /// (whisper.cpp), faster (faster-whisper), or openai (OpenAI Whisper).
    /// In-process when its model is installed, else the first program
    /// found.
    #[arg(long)]
    pub engine: Option<String>,
    /// The in-process model's folder (encoder, decoder, tokenizer.json);
    /// default `<data>/whisper/rten/<model>`, model `base.en`.
    #[arg(long)]
    pub model_dir: Option<PathBuf>,
    /// Print how long loading, the model, and the whole run took.
    #[arg(long)]
    pub timings: bool,
    /// The Whisper program to run (its engine is guessed from the name,
    /// or given with --engine).
    #[arg(long)]
    pub program: Option<PathBuf>,
    /// whisper.cpp's model file (ggml-*.bin), when it is not in a models
    /// folder.
    #[arg(long)]
    pub model_file: Option<PathBuf>,
    /// The spoken language (en, de, ...); detected when not given.
    #[arg(long)]
    pub language: Option<String>,
    /// Start each segment on its own line with its time, `[mm:ss]`.
    #[arg(long)]
    pub timestamps: bool,
    /// Apply spoken commands ("new line", "period", "open quote", ...).
    #[arg(long)]
    pub commands: bool,
    /// Write the transcript to this file instead of printing it.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// List the Whisper programs found and exit.
    #[arg(long)]
    pub list: bool,
    /// Print JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct Report<'a> {
    engine: WhisperEngine,
    program: &'a std::path::Path,
    model: WhisperModel,
    text: &'a str,
    transcript: &'a Transcript,
}

fn list(json: bool) -> anyhow::Result<()> {
    let found = detect();
    if json {
        println!("{}", serde_json::to_string_pretty(&found)?);
        return Ok(());
    }
    if found.is_empty() {
        println!(
            "No Whisper program found. Install whisper.cpp (whisper-cli), faster-whisper (whisper-ctranslate2), or OpenAI Whisper (whisper)."
        );
    }
    for d in &found {
        println!("{}: {}", d.engine.display_name(), d.program.display());
    }
    println!("Models: {}", WHISPER_MODELS.join(", "));
    Ok(())
}

fn config(args: &Args) -> anyhow::Result<WhisperConfig> {
    let model = match &args.model {
        Some(m) => WhisperModel::parse(m).with_context(|| {
            format!(
                "Unknown model size {m:?}; choose from {}",
                WHISPER_MODELS.join(", ")
            )
        })?,
        None => WhisperModel::default(),
    };
    let engine = match &args.engine {
        Some(e) => Some(
            WhisperEngine::parse(e)
                .with_context(|| format!("Unknown engine {e:?}; choose cpp, faster, or openai"))?,
        ),
        None => None,
    };
    let mut config = match &args.program {
        Some(program) => {
            let engine = engine
                .or_else(|| WhisperEngine::from_program(program))
                .context("Name the program's engine with --engine cpp, faster, or openai")?;
            WhisperConfig::new(engine, program.clone(), model)
        }
        None => WhisperConfig::detect(engine, model)?,
    };
    config.model_file = args.model_file.clone();
    config.language = args.language.clone();
    if let Ok(paths) = Paths::platform() {
        config.model_dirs.push(paths.data_dir.join("whisper"));
    }
    Ok(config)
}

/// The in-process model's folder, when it should be used: `--engine
/// rten`, or no engine or program named and the model installed.
fn rten_model_dir(args: &Args) -> Option<PathBuf> {
    let asked = args.engine.as_deref() == Some("rten");
    if !asked && (args.engine.is_some() || args.program.is_some()) {
        return None;
    }
    let dir = args.model_dir.clone().or_else(|| {
        let name = args.model.clone().unwrap_or_else(|| "base.en".into());
        Paths::platform()
            .ok()
            .map(|p| p.data_dir.join("whisper").join("rten").join(name))
    })?;
    (asked || RtenWhisperFiles::in_dir(&dir).is_ok()).then_some(dir)
}

#[derive(Serialize)]
struct RtenReport<'a> {
    engine: &'static str,
    model_dir: &'a std::path::Path,
    text: &'a str,
    transcript: &'a Transcript,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u128>,
}

/// `tw dictate` with Whisper in-process.
fn run_rten(args: &Args, dir: PathBuf) -> anyhow::Result<()> {
    let mut config = RtenConfig::new(&dir);
    config.language = args.language.clone();
    let mut d = RtenDictation::new(config)?;
    match &args.file {
        Some(f) => d.start(DictationInput::File(f.clone()))?,
        None => {
            d.start(DictationInput::Capture(Box::new(MicCapture::new())))?;
            eprintln!("Recording. Press Enter to stop.");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            d.stop()?;
        }
    }
    let mut transcript = None;
    for event in d.wait() {
        match &event {
            DictationEvent::Final(t) => transcript = Some(t.clone()),
            DictationEvent::Failed { message } => anyhow::bail!("{message}"),
            DictationEvent::Cancelled => anyhow::bail!("Dictation cancelled"),
            DictationEvent::Partial(seg) if !args.json => eprintln!("{}", seg.text),
            _ => {
                if !args.json
                    && let Some(a) = event.announcement()
                {
                    eprintln!("{a}");
                }
            }
        }
    }
    let transcript = transcript.unwrap_or_default();
    let timings = d.last_timings();
    if args.timings
        && let Some(t) = timings
    {
        eprintln!(
            "Loading {} ms, spectrogram {} ms, encoder {} ms, decoder {} ms, \
             {} ms of speech, {} ms from the end of the input to the text.",
            t.load.as_millis(),
            t.model.features.as_millis(),
            t.model.encode.as_millis(),
            t.model.decode.as_millis(),
            t.speech.as_millis(),
            t.latency.as_millis()
        );
    }
    let mut text = if args.timestamps {
        transcript.text_with_timestamps()
    } else {
        transcript.text()
    };
    if args.commands {
        text = apply_spoken_commands(&text);
    }
    let out = if args.json {
        serde_json::to_string_pretty(&RtenReport {
            engine: "rten",
            model_dir: &dir,
            text: &text,
            transcript: &transcript,
            latency_ms: timings.map(|t| t.latency.as_millis()),
        })?
    } else {
        text
    };
    match &args.out {
        Some(path) => {
            std::fs::write(path, format!("{out}\n")).with_context(|| path.display().to_string())?
        }
        None => println!("{out}"),
    }
    Ok(())
}

/// Runs `tw dictate`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if args.list {
        return list(args.json);
    }
    if let Some(dir) = rten_model_dir(&args) {
        return run_rten(&args, dir);
    }
    let Some(file) = args.file.clone() else {
        anyhow::bail!(
            "Dictating from the microphone needs the in-process Whisper model in {}. Or transcribe a recording with --file AUDIO.",
            Paths::platform()
                .map(|p| p
                    .data_dir
                    .join("whisper")
                    .join("rten")
                    .join("base.en")
                    .display()
                    .to_string())
                .unwrap_or_else(|_| "the data folder".into())
        );
    };
    let config = config(&args)?;
    let (engine, program, model) = (config.engine, config.program.clone(), config.model);
    let mut d = WhisperDictation::new(config);
    d.start(DictationInput::File(file))?;
    let mut transcript = None;
    loop {
        let events = d.poll();
        for event in events {
            match &event {
                DictationEvent::Final(t) => transcript = Some(t.clone()),
                DictationEvent::Failed { message } => anyhow::bail!("{message}"),
                DictationEvent::Cancelled => anyhow::bail!("Dictation cancelled"),
                DictationEvent::Partial(seg) if !args.json => {
                    eprintln!("{}", seg.text);
                }
                _ => {
                    if !args.json
                        && let Some(a) = event.announcement()
                    {
                        eprintln!("{a}");
                    }
                }
            }
        }
        if transcript.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let transcript = transcript.unwrap_or_default();
    let mut text = if args.timestamps {
        transcript.text_with_timestamps()
    } else {
        transcript.text()
    };
    if args.commands {
        text = apply_spoken_commands(&text);
    }
    if args.json {
        let report = Report {
            engine,
            program: &program,
            model,
            text: &text,
            transcript: &transcript,
        };
        let json = serde_json::to_string_pretty(&report)?;
        match &args.out {
            Some(out) => std::fs::write(out, json).with_context(|| out.display().to_string())?,
            None => println!("{json}"),
        }
        return Ok(());
    }
    if let Some(a) = DictationEvent::Final(transcript.clone()).announcement() {
        eprintln!("{a}");
    }
    match &args.out {
        Some(out) => {
            let mut f = std::fs::File::create(out).with_context(|| out.display().to_string())?;
            writeln!(f, "{text}")?;
        }
        None => println!("{text}"),
    }
    Ok(())
}
