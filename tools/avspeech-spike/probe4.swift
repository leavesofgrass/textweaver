// Probe 4 (Agent F): NSSpeechSynthesizer on a background thread.
// (a) Created and driven on a background thread that pumps its own run loop:
//     do willSpeakWord and didFinishSpeaking arrive, and on which thread?
// (b) startSpeaking(_:to:) (to a file): do word callbacks fire, how fast is
//     it, and what file format is written?
// (c) The pitch base property and rate defaults for Reed and Samantha.
// (d) AVSpeechSynthesizer.write buffer format and callback threads.
// Live speech runs at volume 0; files go to a temporary directory.
import AVFoundation
import AppKit
import Foundation

let text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
let reed = "com.apple.eloquence.en-US.Reed"
let samantha = "com.apple.voice.compact.en-US.Samantha"

final class NSDel: NSObject, NSSpeechSynthesizerDelegate {
    var t0 = Date(); var done = false; var ok = false
    var words: [(Int, String, Bool)] = []
    var finishMain = false
    func speechSynthesizer(_ s: NSSpeechSynthesizer, willSpeakWord r: NSRange, of str: String) {
        words.append((Int(Date().timeIntervalSince(t0) * 1000), (str as NSString).substring(with: r), Thread.isMainThread))
    }
    func speechSynthesizer(_ s: NSSpeechSynthesizer, didFinishSpeaking ok: Bool) {
        done = true; self.ok = ok; finishMain = Thread.isMainThread
    }
}

func spin(_ done: () -> Bool, _ secs: Double = 30) {
    let end = Date().addingTimeInterval(secs)
    while !done() && Date() < end { RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.005)) }
}

func nsLive(_ id: String, label: String) {
    let t0 = Date()
    guard let s = NSSpeechSynthesizer(voice: NSSpeechSynthesizer.VoiceName(rawValue: id)) else { print("  cannot create \(id)"); return }
    let createMs = Int(Date().timeIntervalSince(t0) * 1000)
    s.volume = 0
    let d = NSDel(); s.delegate = d
    let pitch = try? s.object(forProperty: .pitchBase)
    print("\n== NS live \(label) \(id) main=\(Thread.isMainThread) create=\(createMs) ms rate=\(s.rate) volume=\(s.volume) pitchBase=\(String(describing: pitch))")
    d.t0 = Date()
    let started = s.startSpeaking(text)
    spin { d.done }
    let total = Int(Date().timeIntervalSince(d.t0) * 1000)
    print("started=\(started) finished=\(d.done) ok=\(d.ok) finishOnMain=\(d.finishMain) total=\(total) ms words=\(d.words.count)")
    for (ms, w, m) in d.words { print("  \(ms) ms '\(w)' main=\(m)") }
}

func nsFile(_ id: String, label: String, rate: Float?) {
    guard let s = NSSpeechSynthesizer(voice: NSSpeechSynthesizer.VoiceName(rawValue: id)) else { return }
    if let r = rate { s.rate = r }
    let d = NSDel(); s.delegate = d
    let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("probe4-\(label).aiff")
    d.t0 = Date()
    let started = s.startSpeaking(text, to: url)
    spin { d.done }
    let ms = Int(Date().timeIntervalSince(d.t0) * 1000)
    let data = (try? Data(contentsOf: url)) ?? Data()
    let head = data.prefix(12).map { String(format: "%02x", $0) }.joined()
    let ascii = String(decoding: data.prefix(12).map { $0 >= 32 && $0 < 127 ? $0 : 46 }, as: UTF8.self)
    print("\n== NS file \(label) \(id) main=\(Thread.isMainThread) rate=\(s.rate) started=\(started) done=\(d.done) \(ms) ms bytes=\(data.count) head=\(head) '\(ascii)' words=\(d.words.count)")
    if let f = try? AVAudioFile(forReading: url) {
        let secs = Double(f.length) / f.fileFormat.sampleRate
        print("  format \(f.fileFormat) frames=\(f.length) duration=\(String(format: "%.3f", secs)) s")
    }
    try? FileManager.default.removeItem(at: url)
}

final class AVProbe: NSObject, AVSpeechSynthesizerDelegate {
    var done = false; var bufDone = false; var samples = 0
    var wordThreads: [Bool] = []; var bufThreads: [Bool] = []
    var fmt = ""
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance u: AVSpeechUtterance) {
        wordThreads.append(Thread.isMainThread)
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) { done = true }
    func run(_ id: String) {
        let s = AVSpeechSynthesizer(); s.delegate = self
        let u = AVSpeechUtterance(string: text); u.voice = AVSpeechSynthesisVoice(identifier: id)
        let thisThread = Thread.current
        var sameThread = true
        s.write(u) { buf in
            if Thread.current != thisThread { sameThread = false }
            self.bufThreads.append(Thread.isMainThread)
            guard let p = buf as? AVAudioPCMBuffer else { return }
            if self.fmt.isEmpty { self.fmt = "\(p.format) common=\(p.format.commonFormat.rawValue) interleaved=\(p.format.isInterleaved) stride=\(p.stride)" }
            if p.frameLength == 0 { self.bufDone = true; return }
            self.samples += Int(p.frameLength)
        }
        spin { self.bufDone && self.done }
        print("\n== AV write \(id) main=\(Thread.isMainThread) done=\(done) samples=\(samples) buffersOnCallingThread=\(sameThread)")
        print("  format: \(fmt)")
        print("  word callbacks on main: \(wordThreads.filter { $0 }.count)/\(wordThreads.count); buffers on main: \(bufThreads.filter { $0 }.count)/\(bufThreads.count)")
    }
}

print("macOS", ProcessInfo.processInfo.operatingSystemVersionString)
nsLive(reed, label: "main thread (warm-up)")
nsLive(reed, label: "main thread")
nsLive(samantha, label: "main thread")
nsFile(reed, label: "reed-default", rate: nil)
nsFile(samantha, label: "samantha-default", rate: nil)
AVProbe().run(reed)

var bgDone = false
let t = Thread {
    nsLive(reed, label: "background thread")
    nsLive(samantha, label: "background thread")
    nsFile(reed, label: "reed-bg", rate: nil)
    AVProbe().run(reed)
    bgDone = true
}
t.start()
// The main thread does NOT pump its run loop while the background thread
// works, so callbacks that need the main run loop would stall.
while !bgDone { Thread.sleep(forTimeInterval: 0.05) }
print("\nbackground thread finished")
