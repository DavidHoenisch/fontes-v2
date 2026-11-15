import { expect, it } from "vitest";
import { highlightLayer, sameSelection } from "./highlightLayers";
import type { NativeSelection, Workspace } from "./native";
const selection: NativeSelection = {
  translation_id: "kjv",
  spans: [
    { verse_id: "one", start: 2, end: 6, quote: "word" },
    { verse_id: "two", start: 0, end: 5, quote: "other" },
  ],
};
it("matches the entire exact selection and edition rather than any one shared span", () => {
  expect(sameSelection(selection, structuredClone(selection))).toBe(true);
  expect(
    sameSelection(selection, { ...selection, translation_id: "web" }),
  ).toBe(false);
  expect(
    sameSelection(selection, { ...selection, spans: [selection.spans[0]] }),
  ).toBe(false);
  expect(
    sameSelection(selection, {
      ...selection,
      spans: [
        selection.spans[0],
        { ...selection.spans[1], end: 4, quote: "othe" },
      ],
    }),
  ).toBe(false);
  expect(sameSelection(selection, null)).toBe(false);
});
it("edits a matching style independently of other markup and named symbols", () => {
  const state = {
    annotations: [
      { id: "normal", start_ref: 1, end_ref: 2 },
      { id: "god", start_ref: 1, end_ref: 2 },
    ],
    highlights: [
      { annotation_id: "normal", style: { kind: "highlighter" }, selection },
      {
        annotation_id: "god",
        style: { kind: "symbol", symbol: "God" },
        selection,
      },
    ],
  } as unknown as Workspace;
  expect(
    highlightLayer(state, selection, 1, 2, { kind: "symbol", symbol: "God" })
      ?.annotation_id,
  ).toBe("god");
  expect(
    highlightLayer(state, selection, 1, 2, { kind: "symbol", symbol: "Faith" }),
  ).toBeUndefined();
  expect(
    highlightLayer(state, selection, 1, 2, { kind: "highlighter" })
      ?.annotation_id,
  ).toBe("normal");
  expect(
    highlightLayer(state, selection, 1, 3, { kind: "highlighter" }),
  ).toBeUndefined();
});

import { overlappingHighlights } from "./highlightLayers";
it("finds partial overlaps in the current edition, whole-range and legacy marks, while excluding adjacent spans and notes", () => {
  const state = {
    session: { translation: "kjv" },
    annotations: [
      { id: "rich", kind: "highlight", start_ref: 1, end_ref: 2 },
      { id: "adjacent", kind: "highlight", start_ref: 1, end_ref: 1 },
      { id: "other-edition", kind: "highlight", start_ref: 1, end_ref: 2 },
      { id: "legacy", kind: "highlight", start_ref: 1, end_ref: 1 },
      { id: "whole", kind: "highlight", start_ref: 1, end_ref: 1 },
      { id: "note", kind: "note", start_ref: 1, end_ref: 1 },
      { id: "elsewhere", kind: "highlight", start_ref: 3, end_ref: 4 },
    ],
    highlights: [
      { annotation_id: "rich", selection },
      {
        annotation_id: "adjacent",
        selection: {
          translation_id: "kjv",
          spans: [{ verse_id: "one", start: 0, end: 3, quote: "abc" }],
        },
      },
      {
        annotation_id: "other-edition",
        selection: { ...selection, translation_id: "web" },
      },
      { annotation_id: "whole", selection: null },
    ],
  } as unknown as Workspace;
  const partial: NativeSelection = {
    translation_id: "kjv",
    spans: [{ verse_id: "one", start: 3, end: 5, quote: "or" }],
  };
  expect(overlappingHighlights(state, partial, 1, 1).map((a) => a.id)).toEqual([
    "rich",
    "legacy",
    "whole",
  ]);
  expect(overlappingHighlights(state, null, 1, 1).map((a) => a.id)).toEqual([
    "rich",
    "adjacent",
    "legacy",
    "whole",
  ]);
  expect(
    overlappingHighlights(
      state,
      { ...partial, spans: [{ ...partial.spans[0], verse_id: "different" }] },
      1,
      1,
    ).map((a) => a.id),
  ).toEqual(["legacy", "whole"]);
});
