use serde_json::{json, Value};

/// A finalized transcript segment handed from the STT session to the translator.
#[derive(Clone, Debug)]
pub struct FinalTranscript {
    pub segment_id: String,
    pub text: String,
    /// epoch ms when the final transcript arrived
    pub ts: u64,
}

/// `session.update` payload for a Realtime transcription session
/// (GA shape, established empirically against the live API 2026-08).
pub fn session_update_payload(model: &str, use_server_vad: bool) -> Value {
    let turn_detection = if use_server_vad {
        json!({
            "type": "server_vad",
            "threshold": 0.5,
            "prefix_padding_ms": 300,
            "silence_duration_ms": 400
        })
    } else {
        Value::Null
    };

    // Note: no `delay` and no `languages` fields — the live API rejects both
    // for this model ("The '…' parameter is not supported for this model").
    // The model auto-detects the spoken language; direction only shapes the
    // translation prompt.
    let transcription = json!({
        "model": model,
        "prompt": "Technical meeting or interview about software engineering. \
                   Speakers may mix languages and use terms like React, \
                   TypeScript, Next.js, API, backend, frontend, deployment, \
                   Docker, Kubernetes, PostgreSQL."
    });

    json!({
        "type": "session.update",
        "session": {
            "type": "transcription",
            "audio": {
                "input": {
                    "format": { "type": "audio/pcm", "rate": 24000 },
                    "transcription": transcription,
                    "turn_detection": turn_detection
                }
            }
        }
    })
}

pub fn append_payload(base64_audio: &str) -> Value {
    json!({ "type": "input_audio_buffer.append", "audio": base64_audio })
}

pub fn commit_payload() -> Value {
    json!({ "type": "input_audio_buffer.commit" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_update_shape() {
        let v = session_update_payload("gpt-live-transcribe", true);
        assert_eq!(v["type"], "session.update");
        assert_eq!(v["session"]["type"], "transcription");
        assert_eq!(v["session"]["audio"]["input"]["format"]["rate"], 24000);
        assert_eq!(
            v["session"]["audio"]["input"]["transcription"]["model"],
            "gpt-live-transcribe"
        );
        // The live API rejects `languages` and `delay` for this model —
        // neither may ever be present.
        assert!(v["session"]["audio"]["input"]["transcription"]
            .get("languages")
            .is_none());
        assert!(v["session"]["audio"]["input"]["transcription"]
            .get("delay")
            .is_none());
        assert_eq!(
            v["session"]["audio"]["input"]["turn_detection"]["type"],
            "server_vad"
        );
    }

    #[test]
    fn manual_mode_disables_turn_detection() {
        let v = session_update_payload("gpt-live-transcribe", false);
        assert!(v["session"]["audio"]["input"]["turn_detection"].is_null());
    }
}
