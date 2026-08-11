//! Voice-activity gate.
//!
//! Silero VAD (via `voice_activity_detector`) runs on 512-sample windows of
//! 16 kHz audio (32 ms). The gate decides which 24 kHz PCM16 audio is worth
//! sending to OpenAI: silence never leaves the machine. A pre-roll ring keeps
//! the ~400 ms before speech onset so word beginnings are not clipped, and a
//! hangover keeps sending through short pauses (which also gives the server's
//! turn detection the trailing silence it needs to finalize a turn).

use std::collections::VecDeque;

use crate::state::Source;

pub const VAD_SAMPLE_RATE: u32 = 16_000;
pub const VAD_WINDOW_16K: usize = 512; // 32 ms
const WINDOW_24K: usize = 768; // same 32 ms at 24 kHz

const ONSET_PROB: f32 = 0.5;
const ONSET_WINDOWS: u32 = 3; // ~96 ms of speech to open the gate
const HANGOVER_PROB: f32 = 0.35;
const HANGOVER_WINDOWS: u32 = 22; // ~700 ms of silence to close the gate
const PREROLL_SAMPLES_24K: usize = 9_600; // 400 ms
const EMIT_EVERY_24K: usize = 2_400; // flush payload every 100 ms
const MAX_SEGMENT_WINDOWS: u32 = 25_000 / 32; // force-split at ~25 s

pub trait VadEngine: Send {
    /// Speech probability (0..1) for one 512-sample 16 kHz window.
    fn predict(&mut self, window: &[f32]) -> f32;
}

pub struct SileroVad(voice_activity_detector::VoiceActivityDetector);

impl SileroVad {
    pub fn new() -> anyhow::Result<Self> {
        let vad = voice_activity_detector::VoiceActivityDetector::builder()
            .sample_rate(VAD_SAMPLE_RATE as i64)
            .chunk_size(VAD_WINDOW_16K)
            .build()?;
        Ok(Self(vad))
    }
}

impl VadEngine for SileroVad {
    fn predict(&mut self, window: &[f32]) -> f32 {
        self.0.predict(window.iter().copied())
    }
}

/// Fallback engine if the ONNX runtime is unavailable: simple energy gate.
pub struct EnergyVad {
    pub threshold: f32,
}

impl VadEngine for EnergyVad {
    fn predict(&mut self, window: &[f32]) -> f32 {
        let rms = crate::audio::resample::rms(window);
        if rms > self.threshold {
            1.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SegmentEvent {
    Start { id: String },
    /// 24 kHz mono PCM16 payload for the OpenAI session.
    Audio { id: String, pcm: Vec<i16> },
    End { id: String },
}

#[derive(PartialEq)]
enum GateState {
    Idle,
    Speech,
}

pub struct VadGate {
    engine: Box<dyn VadEngine>,
    source: Source,
    state: GateState,
    buf16: Vec<f32>,
    buf24: Vec<i16>,
    preroll: VecDeque<i16>,
    pending: Vec<i16>,
    consec_speech: u32,
    consec_silence: u32,
    windows_in_segment: u32,
    seg_seq: u64,
    current_id: String,
}

impl VadGate {
    pub fn new(engine: Box<dyn VadEngine>, source: Source) -> Self {
        Self {
            engine,
            source,
            state: GateState::Idle,
            buf16: Vec::new(),
            buf24: Vec::new(),
            preroll: VecDeque::with_capacity(PREROLL_SAMPLES_24K),
            pending: Vec::new(),
            consec_speech: 0,
            consec_silence: 0,
            windows_in_segment: 0,
            seg_seq: 0,
            current_id: String::new(),
        }
    }

    /// Feed matching chunks of 16 kHz f32 (VAD branch) and 24 kHz i16
    /// (payload branch); returns any segment events that became ready.
    pub fn push(&mut self, s16: &[f32], s24: &[i16]) -> Vec<SegmentEvent> {
        self.buf16.extend_from_slice(s16);
        self.buf24.extend_from_slice(s24);

        let mut events = Vec::new();
        while self.buf16.len() >= VAD_WINDOW_16K && self.buf24.len() >= WINDOW_24K {
            let w16: Vec<f32> = self.buf16.drain(..VAD_WINDOW_16K).collect();
            let w24: Vec<i16> = self.buf24.drain(..WINDOW_24K).collect();
            let prob = self.engine.predict(&w16);
            self.step(prob, &w24, &mut events);
        }
        events
    }

    fn step(&mut self, prob: f32, w24: &[i16], events: &mut Vec<SegmentEvent>) {
        match self.state {
            GateState::Idle => {
                for &s in w24 {
                    if self.preroll.len() == PREROLL_SAMPLES_24K {
                        self.preroll.pop_front();
                    }
                    self.preroll.push_back(s);
                }
                if prob > ONSET_PROB {
                    self.consec_speech += 1;
                } else {
                    self.consec_speech = 0;
                }
                if self.consec_speech >= ONSET_WINDOWS {
                    self.seg_seq += 1;
                    self.current_id = format!("{}-seg-{}", self.source, self.seg_seq);
                    self.state = GateState::Speech;
                    self.consec_speech = 0;
                    self.consec_silence = 0;
                    self.windows_in_segment = 0;
                    events.push(SegmentEvent::Start {
                        id: self.current_id.clone(),
                    });
                    // Flush pre-roll (which already contains this window's
                    // audio and the onset windows) as the first payload.
                    let pcm: Vec<i16> = self.preroll.drain(..).collect();
                    events.push(SegmentEvent::Audio {
                        id: self.current_id.clone(),
                        pcm,
                    });
                }
            }
            GateState::Speech => {
                self.pending.extend_from_slice(w24);
                self.windows_in_segment += 1;

                if prob < HANGOVER_PROB {
                    self.consec_silence += 1;
                } else {
                    self.consec_silence = 0;
                }

                if self.pending.len() >= EMIT_EVERY_24K {
                    events.push(SegmentEvent::Audio {
                        id: self.current_id.clone(),
                        pcm: std::mem::take(&mut self.pending),
                    });
                }

                let hangover_done = self.consec_silence >= HANGOVER_WINDOWS;
                let force_split = self.windows_in_segment >= MAX_SEGMENT_WINDOWS;
                if hangover_done || force_split {
                    if !self.pending.is_empty() {
                        events.push(SegmentEvent::Audio {
                            id: self.current_id.clone(),
                            pcm: std::mem::take(&mut self.pending),
                        });
                    }
                    events.push(SegmentEvent::End {
                        id: self.current_id.clone(),
                    });
                    self.state = GateState::Idle;
                    self.consec_speech = 0;
                    self.consec_silence = 0;
                    self.preroll.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted engine: returns a fixed probability sequence.
    struct ScriptedVad {
        probs: Vec<f32>,
        i: usize,
    }

    impl VadEngine for ScriptedVad {
        fn predict(&mut self, _: &[f32]) -> f32 {
            let p = self.probs.get(self.i).copied().unwrap_or(0.0);
            self.i += 1;
            p
        }
    }

    fn gate_with(probs: Vec<f32>) -> VadGate {
        VadGate::new(Box::new(ScriptedVad { probs, i: 0 }), Source::System)
    }

    /// Feed n windows of matching 16k/24k audio filled with `value`.
    fn feed(gate: &mut VadGate, windows: usize, value: i16) -> Vec<SegmentEvent> {
        let s16 = vec![0.0f32; VAD_WINDOW_16K * windows];
        let s24 = vec![value; WINDOW_24K * windows];
        gate.push(&s16, &s24)
    }

    fn count_starts(evts: &[SegmentEvent]) -> usize {
        evts.iter()
            .filter(|e| matches!(e, SegmentEvent::Start { .. }))
            .count()
    }

    fn count_ends(evts: &[SegmentEvent]) -> usize {
        evts.iter()
            .filter(|e| matches!(e, SegmentEvent::End { .. }))
            .count()
    }

    #[test]
    fn silence_produces_no_events() {
        let mut g = gate_with(vec![0.0; 100]);
        let evts = feed(&mut g, 100, 0);
        assert!(evts.is_empty());
    }

    #[test]
    fn onset_requires_three_consecutive_speech_windows() {
        // speech, speech, silence, speech, speech — never 3 in a row
        let mut g = gate_with(vec![0.9, 0.9, 0.1, 0.9, 0.9, 0.1]);
        assert!(feed(&mut g, 6, 1).is_empty());

        // 3 in a row opens the gate
        let mut g = gate_with(vec![0.9, 0.9, 0.9]);
        let evts = feed(&mut g, 3, 1);
        assert_eq!(count_starts(&evts), 1);
    }

    #[test]
    fn preroll_flushed_on_start_contains_pre_speech_audio() {
        // 5 silence windows (value 7) then 3 speech windows (value 9)
        let mut probs = vec![0.0; 5];
        probs.extend([0.9, 0.9, 0.9]);
        let mut g = gate_with(probs);
        let mut evts = feed(&mut g, 5, 7);
        evts.extend(feed(&mut g, 3, 9));

        let first_audio = evts
            .iter()
            .find_map(|e| match e {
                SegmentEvent::Audio { pcm, .. } => Some(pcm),
                _ => None,
            })
            .expect("expected an Audio event");
        // Pre-roll must include the silence-valued samples preceding onset.
        assert!(first_audio.contains(&7), "pre-roll audio was clipped");
        assert!(first_audio.contains(&9));
    }

    #[test]
    fn hangover_closes_segment_and_end_follows_start() {
        let mut probs = vec![0.9; 10]; // speech
        probs.extend(vec![0.0; 30]); // silence > 22 windows of hangover
        let mut g = gate_with(probs);
        let mut evts = feed(&mut g, 10, 1);
        evts.extend(feed(&mut g, 30, 0));
        assert_eq!(count_starts(&evts), 1);
        assert_eq!(count_ends(&evts), 1);
        // End must come after Start
        let start_idx = evts
            .iter()
            .position(|e| matches!(e, SegmentEvent::Start { .. }))
            .unwrap();
        let end_idx = evts
            .iter()
            .position(|e| matches!(e, SegmentEvent::End { .. }))
            .unwrap();
        assert!(end_idx > start_idx);
    }

    #[test]
    fn brief_pause_within_hangover_does_not_split_segment() {
        let mut probs = vec![0.9; 5];
        probs.extend(vec![0.0; 10]); // pause shorter than 22 windows
        probs.extend(vec![0.9; 5]);
        probs.extend(vec![0.0; 30]); // real end
        let mut g = gate_with(probs);
        let mut evts = Vec::new();
        evts.extend(feed(&mut g, 5, 1));
        evts.extend(feed(&mut g, 10, 0));
        evts.extend(feed(&mut g, 5, 1));
        evts.extend(feed(&mut g, 30, 0));
        assert_eq!(count_starts(&evts), 1);
        assert_eq!(count_ends(&evts), 1);
    }

    #[test]
    fn long_speech_is_force_split() {
        let windows = MAX_SEGMENT_WINDOWS as usize + 50;
        let mut g = gate_with(vec![0.9; windows + 10]);
        let evts = feed(&mut g, windows + 10, 1);
        assert!(count_ends(&evts) >= 1, "expected a forced split");
    }

    #[test]
    fn segment_ids_are_unique_and_shared_within_segment() {
        let mut probs = vec![0.9; 10];
        probs.extend(vec![0.0; 30]);
        probs.extend(vec![0.9; 10]);
        probs.extend(vec![0.0; 30]);
        let mut g = gate_with(probs);
        let mut evts = Vec::new();
        evts.extend(feed(&mut g, 10, 1));
        evts.extend(feed(&mut g, 30, 0));
        evts.extend(feed(&mut g, 10, 1));
        evts.extend(feed(&mut g, 30, 0));

        let ids: Vec<&String> = evts
            .iter()
            .filter_map(|e| match e {
                SegmentEvent::Start { id } => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }
}
