//! `tw dictate`: transcribe speech with Whisper. Owner: Agent J (wave 2).

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use serde::Serialize;
use textweaver_app::store::Paths;
use textweaver_dictation::{
    Dictation, DictationEvent, DictationInput, Transcript, WHISPER_MODELS, WhisperConfig,
    WhisperDictation, WhisperEngine, WhisperModel, apply_spoken_commands, detect,
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
    /// Which Whisper program: cpp (whisper.cpp), faster (faster-whisper),
    /// or openai (OpenAI Whisper). The first one found by default.
    #[arg(long)]
    pub engine: Option<String>,
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
    /// Start each segment on its own line with its time, [mm:ss].
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

/// Runs `tw dictate`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if args.list {
        return list(args.json);
    }
    let Some(file) = args.file.clone() else {
        anyhow::bail!(
            "Dictating from the microphone is not available in tw yet. Transcribe a recording with --file AUDIO."
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
