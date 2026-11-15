import type { NativeSelection, Workspace } from "./native";

export function sameSelection(
  a: NativeSelection | null,
  b: NativeSelection | null,
) {
  if (!a || !b) return a === b;
  return (
    a.translation_id === b.translation_id &&
    a.spans.length === b.spans.length &&
    a.spans.every((span, i) => {
      const other = b.spans[i];
      return (
        span.verse_id === other.verse_id &&
        span.start === other.start &&
        span.end === other.end &&
        span.quote === other.quote
      );
    })
  );
}

// Each markup kind, and each named study symbol, owns its own editable layer.
export function highlightLayer(
  state: Workspace,
  selection: NativeSelection | null,
  start: number,
  end: number,
  style: { kind: string; symbol?: string | null },
) {
  return state.highlights.find((h) => {
    const annotation = state.annotations.find((a) => a.id === h.annotation_id);
    return (
      annotation?.start_ref === start &&
      annotation.end_ref === end &&
      h.style.kind === style.kind &&
      (style.kind !== "symbol" || h.style.symbol === style.symbol) &&
      sameSelection(h.selection, selection)
    );
  });
}

// Removal may start anywhere inside an existing mark. Exact selection equality
// remains necessary for Apply, but would make touch reselection impossible here.
export function overlappingHighlights(
  state: Workspace,
  selection: NativeSelection | null,
  start: number,
  end: number,
) {
  return state.annotations.filter((annotation) => {
    if (
      annotation.kind !== "highlight" ||
      annotation.start_ref > end ||
      annotation.end_ref < start
    )
      return false;
    const rich = state.highlights.find(
      (h) => h.annotation_id === annotation.id,
    );
    if (!rich?.selection) return true;
    if (rich.selection.translation_id !== state.session.translation)
      return false;
    if (!selection) return true;
    return (
      selection.translation_id === rich.selection.translation_id &&
      rich.selection.spans.some((saved) =>
        selection.spans.some(
          (span) =>
            saved.verse_id === span.verse_id &&
            saved.start < span.end &&
            span.start < saved.end,
        ),
      )
    );
  });
}
