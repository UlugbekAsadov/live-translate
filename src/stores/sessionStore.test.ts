import { beforeEach, describe, expect, it } from "vitest";
import { selectSegments, useSessionStore } from "./sessionStore";

const store = () => useSessionStore.getState();

describe("sessionStore", () => {
  beforeEach(() => {
    store().clear();
  });

  it("creates a segment on first partial and updates it in place", () => {
    store().applyPartial({ source: "system", segmentId: "s-1", text: "Hel", ts: 100 });
    store().applyPartial({ source: "system", segmentId: "s-1", text: "Hello", ts: 120 });

    const segs = selectSegments(store());
    expect(segs).toHaveLength(1);
    expect(segs[0].text).toBe("Hello");
    expect(segs[0].final).toBe(false);
    expect(segs[0].ts).toBe(100); // first-seen timestamp is kept
  });

  it("replaces partial with final by segmentId", () => {
    store().applyPartial({ source: "system", segmentId: "s-1", text: "Hel", ts: 100 });
    store().applyFinal({ source: "system", segmentId: "s-1", text: "Hello there", ts: 300 });

    const segs = selectSegments(store());
    expect(segs).toHaveLength(1);
    expect(segs[0].final).toBe(true);
    expect(segs[0].text).toBe("Hello there");
  });

  it("appends translation deltas then finalizes with latency", () => {
    store().applyFinal({ source: "system", segmentId: "s-1", text: "Hi", ts: Date.now() - 500 });
    store().applyTranslationDelta({ source: "system", segmentId: "s-1", delta: "Sa" });
    store().applyTranslationDelta({ source: "system", segmentId: "s-1", delta: "lom" });
    expect(store().segments["s-1"].translation).toBe("Salom");

    store().applyTranslationFinal({
      source: "system",
      segmentId: "s-1",
      text: "Salom!",
      targetLang: "uz",
    });
    const seg = store().segments["s-1"];
    expect(seg.translation).toBe("Salom!");
    expect(seg.translationFinal).toBe(true);
    expect(store().lastLatencyMs).toBeGreaterThanOrEqual(500);
  });

  it("ignores translation events for unknown segments", () => {
    store().applyTranslationDelta({ source: "mic", segmentId: "nope", delta: "x" });
    expect(selectSegments(store())).toHaveLength(0);
  });

  it("interleaves segments from both sources in arrival order", () => {
    store().applyPartial({ source: "system", segmentId: "s-1", text: "a", ts: 1 });
    store().applyPartial({ source: "mic", segmentId: "m-1", text: "b", ts: 2 });
    store().applyPartial({ source: "system", segmentId: "s-2", text: "c", ts: 3 });
    expect(selectSegments(store()).map((s) => s.id)).toEqual(["s-1", "m-1", "s-2"]);
  });

  it("caps history at 200 segments, dropping oldest", () => {
    for (let i = 0; i < 210; i++) {
      store().applyPartial({ source: "system", segmentId: `s-${i}`, text: "x", ts: i });
    }
    const segs = selectSegments(store());
    expect(segs).toHaveLength(200);
    expect(segs[0].id).toBe("s-10");
    expect(store().segments["s-0"]).toBeUndefined();
  });

  it("clear() empties everything", () => {
    store().applyPartial({ source: "system", segmentId: "s-1", text: "a", ts: 1 });
    store().clear();
    expect(selectSegments(store())).toHaveLength(0);
    expect(store().lastLatencyMs).toBeNull();
  });
});
