import type { CSSProperties } from "react";
export const markColors = [
  ["Yellow", "#c59819"],
  ["Blue", "#3484d7"],
  ["Green", "#429b46"],
  ["Orange", "#da7e20"],
  ["Pink", "#da559a"],
  ["Purple", "#9865d3"],
  ["Red", "#d54542"],
  ["Brown", "#9e7147"],
  ["Gray", "#8b8f97"],
] as const;
export const markKinds = [
  "Highlighter",
  "Text color",
  "Underline",
  "Double underline",
  "Triple underline",
  "Wavy underline",
  "Box",
  "Brackets",
  "Bold",
  "Italic",
  "Strikethrough",
  "Margin bar",
];
export const studySymbols = [
  "God",
  "Jesus Christ",
  "Holy Spirit",
  "Gospel",
  "Grace",
  "Faith",
  "Covenant",
  "Prayer",
  "Law",
  "Kingdom",
  "Promise",
  "Sin",
  "Repentance",
  "Redemption",
  "Eternal life",
  "Question",
  "Important",
  "Bless",
  "Curse",
  "Coming of the Lord",
  "Day of the Lord",
  "Atonement",
  "Believe",
  "Blood",
  "Cry",
];
export type MarkStyle = { kind: string; color: string; symbol?: string };
export const defaultMark: MarkStyle = { kind: "Highlighter", color: "#c59819" };
export function markCSS(style: MarkStyle): CSSProperties {
  const c = style.color;
  const base: CSSProperties = {
    boxDecorationBreak: "clone",
    WebkitBoxDecorationBreak: "clone",
    borderRadius: 2,
  };
  switch (style.kind) {
    case "Text color":
      return { ...base, color: c };
    case "Underline":
      return {
        ...base,
        textDecoration: `underline ${c} 2px`,
        textUnderlineOffset: 4,
      };
    case "Double underline":
      return {
        ...base,
        textDecoration: `underline double ${c} 3px`,
        textUnderlineOffset: 4,
      };
    case "Triple underline":
      return {
        ...base,
        textDecoration: `underline double ${c} 3px`,
        textUnderlineOffset: 4,
        borderBottom: `1px solid ${c}`,
        paddingBottom: 4,
      };
    case "Wavy underline":
      return {
        ...base,
        textDecoration: `underline wavy ${c} 2px`,
        textUnderlineOffset: 4,
      };
    case "Box":
      return { ...base, border: `1px solid ${c}`, padding: "1px 3px" };
    case "Brackets":
      return {
        ...base,
        borderLeft: `2px solid ${c}`,
        borderRight: `2px solid ${c}`,
        padding: "1px 4px",
      };
    case "Bold":
      return { ...base, fontWeight: 700 };
    case "Italic":
      return { ...base, fontStyle: "italic" };
    case "Strikethrough":
      return { ...base, textDecoration: `line-through ${c} 2px` };
    case "Margin bar":
      return { ...base, borderLeft: `3px solid ${c}`, paddingLeft: 6 };
    case "Study symbol": {
      const symbols: Record<string, string> = {
        God: "△",
        "Jesus Christ": "✝",
        "Holy Spirit": "♧",
        Gospel: "▷",
        Grace: "◇",
        Faith: "◇",
        Covenant: "□",
        Prayer: "♧",
        Law: "⌑",
        Kingdom: "♛",
        Promise: "☆",
        Sin: "×",
        Repentance: "↶",
        Redemption: "⌁",
        "Eternal life": "∞",
        Question: "?",
        Important: "!",
        Bless: "⌁",
        Curse: "⌁",
        "Coming of the Lord": "⌁",
        "Day of the Lord": "◇",
        Atonement: "⌒",
        Believe: "⌑",
        Blood: "♦",
        Cry: "◌",
      };
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="28" height="28"><text x="14" y="22" text-anchor="middle" font-size="25" fill="${c}" fill-opacity=".35">${symbols[style.symbol || ""] || "◇"}</text></svg>`;
      return {
        ...base,
        backgroundColor: `${c}20`,
        backgroundImage: `url("data:image/svg+xml,${encodeURIComponent(svg)}")`,
        backgroundRepeat: "repeat",
        borderBottom: `2px solid ${c}`,
        padding: "2px 0",
      };
    }
    default:
      return { ...base, backgroundColor: `${c}50` };
  }
}
