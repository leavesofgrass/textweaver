// Probe 7 (Agent F): the timeline of AVSpeechSynthesizer.write() callbacks,
// to see how macOS 14 differs from 15: when buffers, word callbacks, the
// empty final buffer, and didFinish arrive, and what isSpeaking says. The
// main thread runs its run loop (the callbacks need it). Writes no audio.
import AVFoundation
import Foundation

let passage = "The library opened early on Monday, and the students came in from the rain. They found their seats, set down their bags, and began to read the chapters their teacher had assigned. Some read quietly, others listened with headphones, and a few took careful notes in the margins of their books before the bell rang at noon."
let ns = passage as NSString
let t0 = Date()
func ms() -> Int { Int(Date().timeIntervalSince(t0) * 1000) }

final class Probe: NSObject, AVSpeechSynthesizerDelegate {
    var samples = 0; var buffers = 0; var rate = 0.0
    var words = 0; var finished = false; var emptyBuffer = false
    var log: [String] = []
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance u: AVSpeechUtterance) {
        words += 1
        let audioMs = rate > 0 ? Int(Double(samples) / rate * 1000) : -1
        log.append("\(ms()) ms word \(words) '\(ns.substring(with: r))' at sample \(samples) (\(audioMs) ms of audio), after \(buffers) buffers")
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) {
        finished = true; log.append("\(ms()) ms didFinish (words \(words), buffers \(buffers))")
    }
    func run(_ id: String) {
        let s = AVSpeechSynthesizer(); s.delegate = self
        let u = AVSpeechUtterance(string: passage); u.voice = AVSpeechSynthesisVoice(identifier: id)
        s.write(u) { buf in
            guard let p = buf as? AVAudioPCMBuffer else { self.log.append("\(ms()) ms non-PCM buffer"); return }
            if p.frameLength == 0 { self.emptyBuffer = true; self.log.append("\(ms()) ms empty buffer"); return }
            self.rate = p.format.sampleRate; self.buffers += 1; self.samples += Int(p.frameLength)
            if self.buffers <= 3 || self.buffers % 100 == 0 { self.log.append("\(ms()) ms buffer \(self.buffers) frames \(p.frameLength) total \(self.samples)") }
        }
        var speaking = s.isSpeaking
        log.append("\(ms()) ms write called, isSpeaking \(speaking)")
        let end = Date().addingTimeInterval(25)
        var lastWords = 0; var quietSince = Date()
        while Date() < end {
            RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.005))
            if s.isSpeaking != speaking { speaking = s.isSpeaking; log.append("\(ms()) ms isSpeaking \(speaking) (words \(words), buffers \(buffers))") }
            if words != lastWords { lastWords = words; quietSince = Date() }
            if (finished && emptyBuffer) || Date().timeIntervalSince(quietSince) > 6 { break }
        }
        print("== \(id): words \(words), buffers \(buffers), audio \(rate > 0 ? Int(Double(samples) / rate * 1000) : 0) ms, emptyBuffer \(emptyBuffer), didFinish \(finished)")
        print(log.joined(separator: "\n"))
    }
}

print("macOS", ProcessInfo.processInfo.operatingSystemVersionString)
Probe().run("com.apple.eloquence.en-US.Reed")
