# Live Translate — Real-Time AI Meeting Interpreter (EN ↔ UZ)

A Windows 10/11 desktop app that listens to your computer's audio (Google Meet,
Teams, Zoom, YouTube…) and/or your microphone, transcribes English and Uzbek
speech in real time with OpenAI's Realtime API, translates it naturally
(technical terms stay in English), and shows the result in a small
always-on-top floating overlay you can keep above Google Meet.

```
Google Meet / Teams / Zoom
        ↓
Windows system audio (WASAPI loopback)      Microphone (optional)
        ↓                                        ↓
   Local VAD gate  — silence never leaves your machine
        ↓
OpenAI Realtime transcription (streaming WebSocket, partials < 1 s)
        ↓
Translation (gpt-4o-mini by default, streaming)
        ↓
Floating overlay (Interview Mode) + Dashboard history
```

Directions: **English → Uzbek**, **Uzbek → English**, **Auto-detect → target** —
independently per audio source (e.g. interviewer EN→UZ on system audio, you
UZ→EN on the microphone).

---

## Tech stack

- **Tauri 2** + **React 18** + **TypeScript** + Vite + Tailwind 4 (two windows:
  dashboard + transparent overlay)
- **Rust** backend: cpal (WASAPI loopback + mic), Silero VAD
  (`voice_activity_detector`), tokio, tokio-tungstenite (Realtime WS),
  reqwest (streaming translation), keyring (Windows Credential Manager)
- **OpenAI**: `gpt-live-transcribe` (STT, configurable) +
  `gpt-4o-mini` (translation, configurable)

## Requirements

- Windows 10 or 11 (the app also runs on macOS for development, but system-audio
  loopback capture is Windows-only)
- [Node.js 20+](https://nodejs.org)
- [Rust (rustup)](https://rustup.rs) — stable toolchain
- Microsoft Visual Studio C++ Build Tools (rustup will prompt) and
  [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
  (preinstalled on Windows 11)
- An OpenAI API key with Realtime API access

## Development setup (Windows)

```powershell
git clone <this-repo>
cd live-translate
npm install
npm run tauri dev
```

The first Rust build takes several minutes (it also downloads the ONNX runtime
used by the Silero voice-activity model).

### OpenAI API key

Open **Settings → OpenAI API Key**, paste your `sk-…` key, press **Save**, then
**Test**. The key is stored in **Windows Credential Manager**
(`cmdkey /list` shows a `live-translate` entry) — never in a file, never in
frontend code, and it is read by the Rust backend only when a pipeline starts.
During development you may also pre-seed it with any keyring tool under service
`live-translate`, user `openai_api_key`.

### Audio configuration

- **System audio** is captured with WASAPI *loopback*: the app opens your
  playback device (speakers/headphones) in loopback mode, so it hears exactly
  what you hear — no "Stereo Mix", no virtual cables, and completely
  independent of the microphone. Pick a specific output device on the
  Dashboard or leave "Default device".
- **Microphone** is a normal capture stream, toggleable separately.
- Everything is resampled internally (48 kHz / 44.1 kHz devices both fine) to
  16 kHz for the local VAD and 24 kHz PCM16 for OpenAI.
- The local Silero VAD gates the stream: **silence is never sent to OpenAI**,
  which keeps cost at ~$0.017 per minute of actual speech.

## Using it in an interview

1. Enable **System Audio**, direction *Auto Detect → Uzbek*.
2. (Optional) enable **Microphone**, direction *Auto Detect → English*.
3. Press **Start Translation**, then **Overlay** (or `Ctrl+Shift+O`).
4. Drag the overlay somewhere unobtrusive over Meet; it stays on top.
5. Interview Mode (default) shows the latest translation large with the last
   two dimmed above it; partial transcripts appear while the speaker is still
   talking.

Default global shortcuts (all configurable in Settings, and they work while
Meet has focus):

| Shortcut | Action |
| --- | --- |
| `Ctrl+Shift+O` | Show / hide overlay |
| `Ctrl+Shift+S` | Start / stop translation |
| `Ctrl+Shift+D` | Swap translation direction |
| `Ctrl+Shift+P` | Pause / resume |
| `Ctrl+Shift+X` | Clear history |

History lives only in memory (capped at 200 segments); audio is never stored.
**Clear** wipes it everywhere at once.

> ⚠️ **Screen sharing:** if you share your *entire screen*, the overlay is
> visible to other participants. Share a window/tab instead, or hide the
> overlay while sharing. (Excluding the overlay from capture via
> `SetWindowDisplayAffinity` is on the roadmap.)

## Building the Windows installer

```powershell
npm run tauri build
```

Outputs (in `src-tauri/target/release/bundle/`):

- `nsis/Live Translate_0.1.0_x64-setup.exe` — per-user NSIS installer
  (recommended; no admin rights needed)
- `msi/Live Translate_0.1.0_x64_en-US.msi`

**ONNX runtime note:** the Silero VAD uses `ort`, which places
`onnxruntime*.dll` next to the built exe (`src-tauri/target/release/`). If the
*installed* app logs "Silero VAD unavailable, falling back to energy gate" at
startup, copy that DLL into `src-tauri/` and add it to
`bundle > resources` in `tauri.conf.json` so the installer ships it. The app
still works with the energy-gate fallback, just with a less accurate VAD.

**SmartScreen:** unsigned installers trigger "Windows protected your PC" —
click *More info → Run anyway*, or buy a code-signing certificate / use Azure
Trusted Signing before distributing.

## Testing

Automated:

```powershell
npm test                  # frontend: stores, settings merge, shortcut format
cd src-tauri
cargo test                # Rust: VAD gate, resampler, SSE parser, coalescing,
                          #       backoff, replay buffer, session payload shape
```

Manual checklist (run on Windows 10 and 11):

1. **Meters** — play YouTube → system meter moves; speak → mic meter moves,
   independently.
2. **EN speech** (YouTube tech talk) → partial transcript < 1 s, natural Uzbek
   with React/Docker/API untranslated.
3. **UZ speech** → English translation; auto mode resolves mixed sentences.
4. **Google Meet call** — overlay readable over a maximized Meet window.
5. **Fast / slow speech, background noise, multiple speakers.**
6. **Direction swap** (`Ctrl+Shift+D`) applies to the next segment.
7. **Kill Wi-Fi mid-sentence** → status "Reconnecting…", speech buffered
   (~20 s) and transcribed after recovery.
8. **Wrong API key** → clear error pointing to Settings, no retry loop.
9. **Unplug a Bluetooth headset mid-call** → capture retries, then falls back
   to the default device.
10. **All five shortcuts** while Meet has focus; conflicting combos show red in
    Settings.
11. **Overlay** — drag, resize, opacity, font size; position survives restart.
12. **45-minute session** — session recycling is invisible; CPU stays low
    (single-digit % on a mid-range machine).
13. **Installer** on a clean VM: install → key → live translation end-to-end;
    uninstall is clean (the key intentionally stays in Credential Manager).

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| System meter never moves | Wrong output device selected — pick the device you actually hear audio through, or "Default device". Note: some exclusive-mode audio apps (ASIO players) block loopback; meeting apps don't. |
| "no audio device" on start | No default device, or the named device is gone. Re-select on the Dashboard. |
| No transcripts, status stuck "Listening" | Speech never trips the VAD (very low volume) — raise system volume; or the API key lacks Realtime access — press **Test** in Settings. |
| `invalid_key` error | Key rejected by OpenAI. Re-paste it in Settings; check for spaces. |
| "Reconnecting…" loops forever | Network/proxy blocks `wss://api.openai.com`. Check firewall; corporate proxies must allow WebSockets. |
| Translations slow / "rate_limit" toast | OpenAI rate limits on your account tier; the app retries with backoff and skips a segment only after 3 failures. |
| Overlay invisible but "shown" | Some GPU/driver combos mishandle transparency — disable "Transparency effects" in Windows personalization settings, or check that the overlay isn't parked on a disconnected monitor (delete `overlay-geometry.json` in `%APPDATA%\com.livetranslate.app`). |
| Uzbek transcription quality is poor | Try `gpt-transcribe` in Settings → Models, speak closer to the mic, or fix the direction instead of auto. Whisper-family Uzbek support is improving but weaker than English. |
| "Silero VAD unavailable" in logs | ONNX runtime DLL missing next to the exe — see the ONNX note above. App still works with the energy-gate fallback. |
| Shortcut doesn't fire | Combination owned by another app (red field in Settings) — pick another combo. |
| Logs | `%APPDATA%\com.livetranslate.app\logs\` (Windows), console in dev. Logs never contain your API key; transcripts appear only at debug level. |

## Architecture

```
src/                        React frontend (main window + overlay window)
  pages/                    Dashboard, Settings
  windows/overlay/          OverlayApp, FullMode, InterviewMode, controls
  components/               reusable UI pieces
  services/ipc.ts           typed invoke() wrappers (single source of truth)
  services/eventBus.ts      backend events → zustand stores
  services/controller.ts    start/stop/pause/swap logic shared with shortcuts
  stores/                   session (segments), status, settings
  types/ipc.ts              IPC contract — mirrored by src-tauri/src/events.rs

src-tauri/src/
  audio/capture.rs          cpal streams; WASAPI loopback = output device
                            opened as input; !Send stream owned by its thread
  audio/resample.rs         downmix + stateful linear resampler (48k/44.1k → 16k/24k)
  audio/vad.rs              Silero VAD gate: pre-roll, onset, hangover, force-split
  audio/pipeline.rs         per-source task graph + device-loss recovery
  openai/realtime.rs        Realtime WS session, reconnect + replay, recycle
  openai/translate.rs       streaming chat-completions translator, coalescing queue
  security/keys.rs          Windows Credential Manager via keyring
  shortcuts/                global shortcuts → `shortcut:action` events
  commands/                 thin #[tauri::command] wrappers
```

Design notes:

- **Latency**: local VAD opens the gate ~100 ms after speech onset (with 400 ms
  of pre-roll so no words are clipped); OpenAI streams partial transcripts
  while the speaker talks; translation streams ~0.3–0.5 s after a segment
  finalizes. Target: partial text on screen in under a second.
- **Cost/perf**: silence is filtered locally; audio is chunked (~100 ms
  payloads over one WebSocket, not per-request HTTP); one in-flight
  translation per source with coalescing and dedupe; the UI thread never does
  audio or network work.
- **Resilience**: WS reconnect with exponential backoff + jitter and ~20 s
  audio replay; translation retries honoring `Retry-After`; capture respawn on
  device loss; proactive session recycle at 25 min.
- **Security**: API key only in Credential Manager and Rust memory; no audio
  stored; history is in-memory only; nothing sensitive in logs.
