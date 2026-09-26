// Probe 5 (Agent F): where do Apple's speech callbacks run?
//
// Probe 4 found that when the main thread sleeps (does not run its run
// loop), a synthesizer created on a background thread that pumps its OWN
// run loop receives no callbacks at all: no NSSpeechSynthesizer words or
// finish, no AVSpeechSynthesizer buffers. But in probe 4 the first speech
// use in the process was on the main thread. Each scenario here runs in a
// fresh process (argument 1) so the first use is on the thread under test.
//
//   ns-bg          NSSpeechSynthesizer on a background thread; main sleeps
//   ns-bg-main     ... main thread runs its run loop
//   ns-bg-dispatch ... main thread calls dispatchMain() (GCD main queue only)
//   ns-bg-file     startSpeaking(_:to:) on a background thread; main sleeps
//   av-bg          AVSpeechSynthesizer.write on a background thread; main sleeps
//   av-bg-main     ... main thread runs its run loop
//   av-bg-dispatch ... main thread calls dispatchMain()
//   ssm-bg         Speech Synthesis Manager C API (SpeakCFString with word
//                  and done callbacks) on a background thread; main sleeps
//
// Live speech runs at volume 0; files go to a temporary directory.
import AVFoundation
import AppKit
import ApplicationServices
import Foundation

let text = "Dr. Smith opened the library at 9:30 a.m."
let reed = "com.apple.eloquence.en-US.Reed"
let scenario = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "ns-bg"
let t0 = Date()
func ms() -> Int { Int(Date().timeIntervalSince(t0) * 1000) }
func thread() -> String { Thread.isMainThread ? "main" : "bg" }

let lock = NSLock()
var log: [String] = []
func note(_ s: String) { lock.lock(); log.append("  \(ms()) ms [\(thread())] \(s)"); lock.unlock() }
var finished = false
func isFinished() -> Bool { lock.lock(); defer { lock.unlock() }; return finished }
func finish() { lock.lock(); finished = true; lock.unlock() }

final class NSDel: NSObject, NSSpeechSynthesizerDelegate {
    func speechSynthesizer(_ s: NSSpeechSynthesizer, willSpeakWord r: NSRange, of str: String) {
        note("word '\((str as NSString).substring(with: r))'")
    }
    func speechSynthesizer(_ s: NSSpeechSynthesizer, didFinishSpeaking ok: Bool) {
        note("didFinishSpeaking \(ok)"); finish()
    }
}

final class AVDel: NSObject, AVSpeechSynthesizerDelegate {
    func speechSynthesizer(_ s: AVSpeechSynthesizer, willSpeakRangeOfSpeechString r: NSRange, utterance u: AVSpeechUtterance) {
        note("word '\((text as NSString).substring(with: r))'")
    }
    func speechSynthesizer(_ s: AVSpeechSynthesizer, didFinish u: AVSpeechUtterance) { note("didFinish"); finish() }
}

// Speech Synthesis Manager callbacks: C function pointers, no captures.
let ssmWord: SpeechWordCFProcPtr = { _, _, str, range in
    note("ssm word '\((str as NSString).substring(with: NSRange(location: range.location, length: range.length)))'")
}
let ssmDone: SpeechDoneProcPtr = { _, _ in note("ssm done"); finish() }

var keep: [AnyObject] = []

func work() {
    note("worker start")
    switch scenario {
    case "ns-bg", "ns-bg-main", "ns-bg-dispatch", "ns-bg-file":
        guard let s = NSSpeechSynthesizer(voice: NSSpeechSynthesizer.VoiceName(rawValue: reed)) else { note("cannot create"); finish(); return }
        let d = NSDel(); s.delegate = d; keep = [s, d]
        if scenario == "ns-bg-file" {
            let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("probe5.aiff")
            note("startSpeaking to file -> \(s.startSpeaking(text, to: url))")
        } else {
            s.volume = 0
            note("startSpeaking -> \(s.startSpeaking(text))")
        }
        var wasSpeaking = s.isSpeaking
        note("isSpeaking \(wasSpeaking)")
        let end = Date().addingTimeInterval(15)
        while !isFinished() && Date() < end {
            RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.01))
            if s.isSpeaking != wasSpeaking { wasSpeaking = s.isSpeaking; note("isSpeaking \(wasSpeaking)") }
        }
    case "av-bg", "av-bg-main", "av-bg-dispatch":
        let s = AVSpeechSynthesizer(); let d = AVDel(); s.delegate = d; keep = [s, d]
        let u = AVSpeechUtterance(string: text); u.voice = AVSpeechSynthesisVoice(identifier: reed)
        var buffers = 0
        s.write(u) { buf in
            buffers += 1
            if buffers == 1 { note("first buffer") }
            if let p = buf as? AVAudioPCMBuffer, p.frameLength == 0 { note("last buffer (\(buffers))") }
        }
        note("write called")
        let end = Date().addingTimeInterval(15)
        while !isFinished() && Date() < end { RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.01)) }
        note("buffers \(buffers)")
    case "ssm-bg":
        var chan: SpeechChannel?
        note("NewSpeechChannel -> \(NewSpeechChannel(nil, &chan))")
        guard let c = chan else { finish(); return }
        note("word cb -> \(SetSpeechProperty(c, kSpeechWordCFCallBack, NSNumber(value: Int(bitPattern: unsafeBitCast(ssmWord, to: UnsafeRawPointer.self)))))")
        note("done cb -> \(SetSpeechProperty(c, kSpeechSpeechDoneCallBack, NSNumber(value: Int(bitPattern: unsafeBitCast(ssmDone, to: UnsafeRawPointer.self)))))")
        note("volume -> \(SetSpeechProperty(c, kSpeechVolumeProperty, NSNumber(value: 0.0)))")
        note("SpeakCFString -> \(SpeakCFString(c, text as CFString, nil))")
        let end = Date().addingTimeInterval(15)
        while !isFinished() && Date() < end { RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.01)) }
        note("SpeechBusy -> \(SpeechBusy())")
    default:
        note("unknown scenario"); finish()
    }
    note("worker end, finished=\(isFinished())")
}

print("macOS", ProcessInfo.processInfo.operatingSystemVersionString, "scenario", scenario)
let worker = Thread {
    work()
    lock.lock(); let lines = log; lock.unlock()
    print(lines.joined(separator: "\n"))
    exit(0)
}
worker.start()
if scenario.hasSuffix("-main") {
    while true { RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05)) }
} else if scenario.hasSuffix("-dispatch") {
    dispatchMain()
} else {
    while true { Thread.sleep(forTimeInterval: 0.05) }
}
