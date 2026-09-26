param([string]$Out, [string]$Skip = 'Eloquence')
# Probe SAPI5 voices visible to this process (64- or 32-bit): word-boundary
# events with character position and audio position. Output goes to WAV
# files in $Out; nothing is played. Voices whose names match the regular
# expression $Skip are skipped (default: Eloquence, not licensed on the
# development machine; scripts/speech-check.ps1 passes its own).
Add-Type -AssemblyName System.Speech
$text = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé."
"process 64-bit: $([Environment]::Is64BitProcess)"
$probe = New-Object System.Speech.Synthesis.SpeechSynthesizer
$names = $probe.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object { $_.VoiceInfo.Name }
$probe.Dispose()
foreach ($name in $names) {
    if ($Skip -and $name -match $Skip) { "`n== $name : skipped (matches $Skip)"; continue }
    $s = New-Object System.Speech.Synthesis.SpeechSynthesizer
    try { $s.SelectVoice($name) } catch { "`n== $name : cannot select"; continue }
    $wav = Join-Path $Out (($name -replace '[^A-Za-z0-9]', '_') + '.wav')
    $s.SetOutputToWaveFile($wav)
    $id = "sp" + [guid]::NewGuid().ToString('N')
    Register-ObjectEvent -InputObject $s -EventName SpeakProgress -SourceIdentifier $id | Out-Null
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $s.Speak($text)
    $ms = $sw.ElapsedMilliseconds
    Start-Sleep -Milliseconds 200
    $ev = @(Get-Event -SourceIdentifier $id | ForEach-Object { $_.SourceEventArgs } | Sort-Object { $_.AudioPosition })
    Unregister-Event $id; Get-Event -SourceIdentifier $id -ErrorAction SilentlyContinue | Remove-Event
    $s.Dispose()
    $bytes = (Get-Item $wav).Length
    "`n== $name : $($ev.Count) word events, synthesized in $ms ms, wav $bytes bytes"
    foreach ($a in $ev) { "  {0,3}+{1,-2} audio {2,5} ms  '{3}'" -f $a.CharacterPosition, $a.CharacterCount, [int]$a.AudioPosition.TotalMilliseconds, $a.Text }
}
