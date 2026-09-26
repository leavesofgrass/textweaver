// Probe 3: (a) in write() mode, do word callbacks interleave with buffer
// callbacks so each word gets a sample offset? (b) does AVSpeechSynthesizer
// work when created and driven on a background thread with its own run loop?
import AVFoundation
import Foundation

let text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
let ns = text as NSString

final class Probe: NSObject, AVSpeechSynthesizerDelegate {
    var samples = 0; var rate = 0.0; var bufDone = false; var done = false
    var words: [(Int, String)] = []
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance u: AVSpeechUtterance) {
        words.append((samples, ns.substring(with: r)))
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) { done = true }
    func run(_ id: String, label: String) {
        let s = AVSpeechSynthesizer(); s.delegate = self
        let u = AVSpeechUtterance(string: text)
        u.voice = AVSpeechSynthesisVoice(identifier: id)
        s.write(u) { buf in
            guard let p = buf as? AVAudioPCMBuffer else { return }
            if p.frameLength == 0 { self.bufDone = true; return }
            self.rate = p.format.sampleRate; self.samples += Int(p.frameLength)
        }
        let end = Date().addingTimeInterval(30)
        while !(bufDone && done) && Date() < end { RunLoop.current.run(until: Date().addingTimeInterval(0.01)) }
        print("\n== \(label) \(id) main=\(Thread.isMainThread) finished=\(done) samples=\(samples) rate=\(rate)")
        for (off, w) in words { print("  \(rate > 0 ? Int(Double(off) / rate * 1000) : -1) ms (sample \(off)) '\(w)'") }
    }
}

Probe().run("com.apple.eloquence.en-US.Reed", label: "main thread")
Probe().run("com.apple.voice.compact.en-US.Samantha", label: "main thread")
var bgDone = false
let t = Thread {
    Probe().run("com.apple.eloquence.en-US.Reed", label: "background thread")
    bgDone = true
}
t.start()
while !bgDone { RunLoop.current.run(until: Date().addingTimeInterval(0.05)) }
