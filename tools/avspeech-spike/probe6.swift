// Probe 6 (Agent F): the Speech Synthesis Manager (the C API underneath
// NSSpeechSynthesizer) on a background thread while the main thread sleeps.
// Probe 5 found that NSSpeechSynthesizer delivers its delegate callbacks
// only through the main run loop, and AVSpeechSynthesizer only through the
// main dispatch queue, while the Speech Synthesis Manager delivers word and
// done callbacks on its own threads. This probe checks what a backend needs:
//
//   attrs   voice attributes (numeric ids for VoiceSpec) for Reed, Samantha
//   voice   select Reed by VoiceSpec, speak, word ranges and first-word latency
//   pause   PauseSpeechAt(kEndOfWord), status while paused, ContinueSpeech
//   stop    StopSpeech mid-sentence: does the done callback fire?
//   file    kSpeechOutputToFileURLProperty: speed, words, file format
//   rate    wpm calibration: rate property vs measured words per minute
//   pitch   the pitch base property and whether setting it takes
//
// Live speech runs at volume 0; files go to a temporary directory.
import AVFoundation
import AppKit
import ApplicationServices
import Foundation

let reed = "com.apple.eloquence.en-US.Reed"
let samantha = "com.apple.voice.compact.en-US.Samantha"
let sentence = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
// 60 words, for rate calibration.
let passage = """
The library opened early on Monday, and the students came in from the rain. \
They found their seats, set down their bags, and began to read the chapters \
their teacher had assigned. Some read quietly, others listened with headphones, \
and a few took careful notes in the margins of their books before the bell rang at noon.
"""
let scenario = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "attrs"
var t0 = Date()
func ms() -> Int { Int(Date().timeIntervalSince(t0) * 1000) }
func thread() -> String { Thread.isMainThread ? "main" : (Thread.current.name?.isEmpty == false ? Thread.current.name! : "other") }

let lock = NSLock()
var log: [String] = []
var spoken: String = sentence
func note(_ s: String) { lock.lock(); log.append("  \(ms()) ms [\(thread())] \(s)"); lock.unlock() }
func flush() { lock.lock(); print(log.joined(separator: "\n")); log = []; lock.unlock() }
var done = false
var words = 0
func isDone() -> Bool { lock.lock(); defer { lock.unlock() }; return done }
func reset() { lock.lock(); done = false; words = 0; lock.unlock(); t0 = Date() }

let wordCB: SpeechWordCFProcPtr = { _, _, str, range in
    lock.lock(); words += 1; lock.unlock()
    if scenario != "rate" {
        note("word \(range.location)+\(range.length) '\((str as NSString).substring(with: NSRange(location: range.location, length: range.length)))'")
    }
}
let doneCB: SpeechDoneProcPtr = { _, _ in note("done"); lock.lock(); done = true; lock.unlock() }

func wait(_ secs: Double) { let end = Date().addingTimeInterval(secs); while !isDone() && Date() < end { Thread.sleep(forTimeInterval: 0.005) } }

// The voice attributes carry VoiceNumericID but no creator, so find the
// VoiceSpec by walking GetIndVoice and matching the id.
func spec(_ id: String) -> VoiceSpec? {
    let started = Date()
    let attrs = NSSpeechSynthesizer.attributes(forVoice: NSSpeechSynthesizer.VoiceName(rawValue: id))
    guard let vid = attrs[NSSpeechSynthesizer.VoiceAttributeKey(rawValue: "VoiceNumericID")] as? NSNumber else { return nil }
    let want = OSType(truncatingIfNeeded: vid.int64Value)
    var n: Int16 = 0
    CountVoices(&n)
    var i: Int16 = 1
    while i <= n {
        var s = VoiceSpec()
        if GetIndVoice(i, &s) == 0 && s.id == want {
            var desc = VoiceDescription()
            let e = GetVoiceDescription(&s, &desc, MemoryLayout<VoiceDescription>.size)
            let name = withUnsafeBytes(of: desc.name) { b in String(decoding: b.dropFirst().prefix(Int(b[0])), as: UTF8.self) }
            note(String(format: "spec for %@: index %d creator %u id %u desc err %d name '%@' lookup %.1f ms", id, Int(i), s.creator, s.id, Int(e), name, Date().timeIntervalSince(started) * 1000))
            return s
        }
        i += 1
    }
    return nil
}

func channel(_ id: String) -> SpeechChannel? {
    guard var s = spec(id) else { note("no VoiceSpec for \(id)"); return nil }
    var chan: SpeechChannel?
    let err = NewSpeechChannel(&s, &chan)
    note("NewSpeechChannel(\(id)) creator=\(s.creator) id=\(s.id) -> \(err)")
    guard let c = chan else { return nil }
    SetSpeechProperty(c, kSpeechWordCFCallBack, NSNumber(value: Int(bitPattern: unsafeBitCast(wordCB, to: UnsafeRawPointer.self))))
    SetSpeechProperty(c, kSpeechSpeechDoneCallBack, NSNumber(value: Int(bitPattern: unsafeBitCast(doneCB, to: UnsafeRawPointer.self))))
    var cur: CFTypeRef?
    if CopySpeechProperty(c, kSpeechCurrentVoiceProperty, &cur) == 0, let v = cur { note("current voice \(v)") }
    return c
}

func prop(_ c: SpeechChannel, _ key: CFString) -> String {
    var out: CFTypeRef?
    let err = CopySpeechProperty(c, key, &out)
    return err == 0 ? "\(out.map { "\($0)" } ?? "nil")" : "error \(err)"
}

func fileDuration(_ url: URL) -> Double {
    guard let f = try? AVAudioFile(forReading: url) else { return -1 }
    return Double(f.length) / f.fileFormat.sampleRate
}

func work() {
    switch scenario {
    case "attrs":
        let all = NSSpeechSynthesizer.availableVoices
        note("availableVoices \(all.count), eloquence \(all.filter { $0.rawValue.contains("eloquence") }.count)")
        for id in [reed, samantha, "com.apple.eloquence.de-DE.Reed"] {
            let a = NSSpeechSynthesizer.attributes(forVoice: NSSpeechSynthesizer.VoiceName(rawValue: id))
            note("\(id):")
            for (k, v) in a.sorted(by: { $0.key.rawValue < $1.key.rawValue }) where k.rawValue != "VoiceSupportedCharacters" && k.rawValue != "VoiceIndividuallySpokenCharacters" {
                note("    \(k.rawValue) = \(v)")
            }
        }
        var n: Int16 = 0
        CountVoices(&n)
        note("CountVoices \(n)")
    case "voice":
        for id in [reed, samantha, reed] {
            guard let c = channel(id) else { continue }
            SetSpeechProperty(c, kSpeechVolumeProperty, NSNumber(value: 0.0))
            note("rate \(prop(c, kSpeechRateProperty)) pitch \(prop(c, kSpeechPitchBaseProperty)) volume \(prop(c, kSpeechVolumeProperty))")
            reset()
            note("SpeakCFString -> \(SpeakCFString(c, sentence as CFString, nil))")
            wait(20)
            flush()
            DisposeSpeechChannel(c)
        }
    case "pause":
        guard let c = channel(reed) else { return }
        SetSpeechProperty(c, kSpeechVolumeProperty, NSNumber(value: 0.0))
        reset()
        SpeakCFString(c, passage as CFString, nil)
        Thread.sleep(forTimeInterval: 1.5)
        note("PauseSpeechAt(word) -> \(PauseSpeechAt(c, Int32(kEndOfWord)))")
        Thread.sleep(forTimeInterval: 0.3)
        note("status \(prop(c, kSpeechStatusProperty))")
        let w = words
        Thread.sleep(forTimeInterval: 1.5)
        note("words during pause: \(words - w); status \(prop(c, kSpeechStatusProperty))")
        note("ContinueSpeech -> \(ContinueSpeech(c))")
        Thread.sleep(forTimeInterval: 1.0)
        note("StopSpeech -> \(StopSpeech(c))")
        wait(3)
        note("done after stop: \(isDone())")
    case "stop":
        guard let c = channel(reed) else { return }
        SetSpeechProperty(c, kSpeechVolumeProperty, NSNumber(value: 0.0))
        reset()
        SpeakCFString(c, sentence as CFString, nil)
        Thread.sleep(forTimeInterval: 0.8)
        note("StopSpeech -> \(StopSpeech(c)); SpeechBusy \(SpeechBusy())")
        wait(3)
        note("done callback after StopSpeech: \(isDone())")
        reset()
        note("speak again -> \(SpeakCFString(c, "Second sentence here." as CFString, nil))")
        wait(10)
        note("second done: \(isDone())")
    case "file":
        for id in [reed, samantha] {
            guard let c = channel(id) else { continue }
            let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("probe6.aiff")
            try? FileManager.default.removeItem(at: url)
            note("output url -> \(SetSpeechProperty(c, kSpeechOutputToFileURLProperty, url as NSURL))")
            reset()
            SpeakCFString(c, sentence as CFString, nil)
            wait(20)
            let data = (try? Data(contentsOf: url)) ?? Data()
            note("done=\(isDone()) words=\(words) bytes=\(data.count) head=\(String(decoding: data.prefix(12).map { $0 >= 32 && $0 < 127 ? $0 : 46 }, as: UTF8.self)) duration=\(fileDuration(url))")
            flush()
            DisposeSpeechChannel(c)
        }
    case "rate":
        let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("probe6-rate.aiff")
        for id in [reed, samantha] {
            for r in [90.0, 150.0, 200.0, 265.0, 350.0, 500.0, 700.0, 900.0] {
                guard let c = channel(id) else { continue }
                try? FileManager.default.removeItem(at: url)
                SetSpeechProperty(c, kSpeechOutputToFileURLProperty, url as NSURL)
                let e = SetSpeechProperty(c, kSpeechRateProperty, NSNumber(value: r))
                let got = prop(c, kSpeechRateProperty)
                reset()
                SpeakCFString(c, passage as CFString, nil)
                wait(60)
                let d = fileDuration(url)
                note(String(format: "RATE %@ set %.0f (err %ld, reads %@) -> %.3f s, %.1f wpm, %ld word callbacks", id, r, Int(e), got, d, 60.0 / d * 60.0, words))
                DisposeSpeechChannel(c)
                lock.lock(); log = log.filter { $0.contains("RATE") }; lock.unlock()
            }
        }
    case "pitch":
        guard let c = channel(reed) else { return }
        let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("probe6-pitch.aiff")
        note("pitch base \(prop(c, kSpeechPitchBaseProperty)) mod \(prop(c, kSpeechPitchModProperty))")
        for p in [32.0, 44.0, 56.0] {
            try? FileManager.default.removeItem(at: url)
            SetSpeechProperty(c, kSpeechOutputToFileURLProperty, url as NSURL)
            note("set pitch \(p) -> \(SetSpeechProperty(c, kSpeechPitchBaseProperty, NSNumber(value: p))) reads \(prop(c, kSpeechPitchBaseProperty))")
            reset()
            SpeakCFString(c, "Hello there." as CFString, nil)
            wait(10)
            // Zero crossings per second of the voiced audio: a rough pitch proxy.
            if let f = try? AVAudioFile(forReading: url), let buf = AVAudioPCMBuffer(pcmFormat: f.processingFormat, frameCapacity: AVAudioFrameCount(f.length)) {
                try? f.read(into: buf)
                var zc = 0; var loud = 0
                if let ch = buf.floatChannelData?[0] {
                    for i in 1..<Int(buf.frameLength) where abs(ch[i]) > 0.02 { loud += 1; if (ch[i] > 0) != (ch[i-1] > 0) { zc += 1 } }
                }
                note(String(format: "pitch %.0f: zero crossings per voiced second %.0f", p, loud > 0 ? Double(zc) / (Double(loud) / f.fileFormat.sampleRate) : 0))
            }
        }
    default:
        note("unknown scenario")
    }
}

print("macOS", ProcessInfo.processInfo.operatingSystemVersionString, "scenario", scenario)
let worker = Thread { work(); flush(); exit(0) }
worker.name = "worker"
worker.start()
while true { Thread.sleep(forTimeInterval: 0.05) }
