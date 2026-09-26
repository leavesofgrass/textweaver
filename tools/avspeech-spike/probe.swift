// Probe: which Apple voices exist, which are Eloquence, and whether
// AVSpeechSynthesizer.write(_:toBufferCallback:toMarkerCallback:) (macOS 14+)
// reports word markers with sample offsets. Writes no audio anywhere.
import AVFoundation
import Foundation

print("macOS", ProcessInfo.processInfo.operatingSystemVersionString)
let voices = AVSpeechSynthesisVoice.speechVoices()
print("total voices:", voices.count)
let eloquence = voices.filter { $0.identifier.lowercased().contains("eloquence") }
print("eloquence voices:", eloquence.count)
for v in eloquence { print("  \(v.identifier) | \(v.name) | \(v.language) | quality \(v.quality.rawValue)") }

let text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
let ns = text as NSString

func probe(_ voice: AVSpeechSynthesisVoice?) {
    print("\n== voice:", voice?.identifier ?? "default")
    let synth = AVSpeechSynthesizer()
    let u = AVSpeechUtterance(string: text)
    u.voice = voice
    var samples = 0
    var rate = 0.0
    var finished = false
    var marks: [(String, Int, String)] = []
    synth.write(u, toBufferCallback: { buf in
        guard let pcm = buf as? AVAudioPCMBuffer else { return }
        if pcm.frameLength == 0 { finished = true; return }
        rate = pcm.format.sampleRate
        samples += Int(pcm.frameLength)
    }, toMarkerCallback: { markers in
        for m in markers {
            let word = m.textRange.location + m.textRange.length <= ns.length ? ns.substring(with: m.textRange) : "?"
            marks.append(("\(m.mark.rawValue)", m.byteSampleOffset, word))
        }
    })
    let deadline = Date().addingTimeInterval(30)
    while !finished && Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.05)) }
    print("finished:", finished, "rate:", rate, "samples:", samples)
    for (kind, off, word) in marks {
        let ms = rate > 0 ? Int(Double(off) / 2.0 / rate * 1000) : -1
        print("  mark \(kind) byteSampleOffset \(off) (~\(ms) ms if 16-bit mono) '\(word)'")
    }
}

probe(eloquence.first { $0.language == "en-US" })
probe(AVSpeechSynthesisVoice(language: "en-US"))
