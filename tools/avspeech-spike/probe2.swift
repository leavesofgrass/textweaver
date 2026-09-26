// Probe 2: word-boundary delivery for Eloquence and Samantha through
// (a) AVSpeechSynthesizer delegate willSpeakRange during speak(),
// (b) the same delegate during write() to a buffer,
// (c) NSSpeechSynthesizer willSpeakWord during startSpeaking().
import AVFoundation
import AppKit
import Foundation

let text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
let ns = text as NSString
func spin(_ done: () -> Bool, _ secs: Double = 30) {
    let end = Date().addingTimeInterval(secs)
    while !done() && Date() < end { RunLoop.current.run(until: Date().addingTimeInterval(0.02)) }
}

final class Del: NSObject, AVSpeechSynthesizerDelegate {
    var t0 = Date(); var done = false; var words: [(Int, String)] = []
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didStart u: AVSpeechUtterance) { t0 = Date() }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance u: AVSpeechUtterance) {
        words.append((Int(Date().timeIntervalSince(t0) * 1000), ns.substring(with: r)))
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) { done = true }
}

func avspeak(_ voice: AVSpeechSynthesisVoice?, write: Bool) {
    print("\n== AVSpeech \(write ? "write" : "speak") voice:", voice?.identifier ?? "nil")
    let s = AVSpeechSynthesizer(); let d = Del(); s.delegate = d
    let u = AVSpeechUtterance(string: text); u.voice = voice
    var samples = 0; var rate = 0.0; var bufDone = false
    if write {
        s.write(u) { buf in
            guard let p = buf as? AVAudioPCMBuffer else { return }
            if p.frameLength == 0 { bufDone = true; return }
            rate = p.format.sampleRate; samples += Int(p.frameLength)
        }
        spin { bufDone && (d.done || true) }
        spin({ d.done }, 2)
        print("buffer samples \(samples) at \(rate) = \(rate > 0 ? Int(Double(samples) / rate * 1000) : 0) ms")
    } else {
        s.speak(u); spin { d.done }
    }
    print("delegate finished: \(d.done), word callbacks: \(d.words.count)")
    for (ms, w) in d.words { print("  \(ms) ms '\(w)'") }
}

final class NSDel: NSObject, NSSpeechSynthesizerDelegate {
    var t0 = Date(); var done = false; var words: [(Int, String)] = []
    func speechSynthesizer(_ s: NSSpeechSynthesizer, willSpeakWord r: NSRange, of str: String) {
        words.append((Int(Date().timeIntervalSince(t0) * 1000), (str as NSString).substring(with: r)))
    }
    func speechSynthesizer(_ s: NSSpeechSynthesizer, didFinishSpeaking ok: Bool) { done = true }
}

func nsspeak(_ id: String) {
    print("\n== NSSpeechSynthesizer voice:", id)
    guard let s = NSSpeechSynthesizer(voice: NSSpeechSynthesizer.VoiceName(rawValue: id)) else { print("  cannot create"); return }
    let d = NSDel(); s.delegate = d; d.t0 = Date()
    s.startSpeaking(text); spin { d.done }
    print("finished: \(d.done), word callbacks: \(d.words.count)")
    for (ms, w) in d.words { print("  \(ms) ms '\(w)'") }
}

let voices = AVSpeechSynthesisVoice.speechVoices()
let eloq = voices.first { $0.identifier == "com.apple.eloquence.en-US.Reed" } ?? voices.first { $0.identifier.contains("eloquence.en-US") }
let sam = AVSpeechSynthesisVoice(language: "en-US")
avspeak(eloq, write: false)
avspeak(eloq, write: true)
avspeak(sam, write: false)
nsspeak(eloq?.identifier ?? "")
nsspeak(sam?.identifier ?? "")
