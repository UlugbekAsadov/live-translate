use serde_json::{json, Value};

use crate::state::Direction;

/// A finalized transcript segment handed from the STT session to the translator.
#[derive(Clone, Debug)]
pub struct FinalTranscript {
    pub segment_id: String,
    pub text: String,
    /// epoch ms when the final transcript arrived
    pub ts: u64,
}

/// `session.update` payload for a Realtime transcription session
/// (GA shape — see the OpenAI realtime-transcription guide).
pub fn session_update_payload(model: &str, direction: Direction, use_server_vad: bool) -> Value {
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

    let mut transcription = json!({
        "model": model,
        "prompt": "Technical meeting or interview about software engineering. \
                   Speakers use English and Uzbek and mix in terms like React, \
                   TypeScript, Next.js, API, backend, frontend, deployment, \
                   Docker, Kubernetes, PostgreSQL.",
        "delay": "low"
    });
    // `uz` is not an accepted hint — omit the field entirely for directions
    // that may carry Uzbek audio (empty hint list) and let the model detect.
    let languages = direction.stt_languages();
    if !languages.is_empty() {
        transcription["languages"] = json!(languages);
    }

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
        let v = session_update_payload("gpt-live-transcribe", Direction::AutoUz, true);
        assert_eq!(v["type"], "session.update");
        assert_eq!(v["session"]["type"], "transcription");
        assert_eq!(v["session"]["audio"]["input"]["format"]["rate"], 24000);
        assert_eq!(
            v["session"]["audio"]["input"]["transcription"]["model"],
            "gpt-live-transcribe"
        );
        // Uzbek-capable directions must NOT send a languages hint at all.
        assert!(v["session"]["audio"]["input"]["transcription"]
            .get("languages")
            .is_none());
        assert_eq!(
            v["session"]["audio"]["input"]["turn_detection"]["type"],
            "server_vad"
        );
    }

    #[test]
    fn manual_mode_disables_turn_detection() {
        let v = session_update_payload("gpt-live-transcribe", Direction::EnUz, false);
        assert!(v["session"]["audio"]["input"]["turn_detection"].is_null());
        let langs = &v["session"]["audio"]["input"]["transcription"]["languages"];
        assert_eq!(langs.as_array().unwrap().len(), 1);
        assert_eq!(langs[0], "en");
    }
}
