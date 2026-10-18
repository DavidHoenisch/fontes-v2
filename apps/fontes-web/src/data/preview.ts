import bookOpenings from "./book-openings.json";
import introductions from "../../../../assets/ui/book-covers/introductions.json";
const art = import.meta.glob("../../../../assets/ui/book-covers/*.svg", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;
const names = [
  "Gen",
  "Exod",
  "Lev",
  "Num",
  "Deut",
  "Josh",
  "Judg",
  "Ruth",
  "1Sam",
  "2Sam",
  "1Kgs",
  "2Kgs",
  "1Chr",
  "2Chr",
  "Ezra",
  "Neh",
  "Esth",
  "Job",
  "Ps",
  "Prov",
  "Eccl",
  "Song",
  "Isa",
  "Jer",
  "Lam",
  "Ezek",
  "Dan",
  "Hos",
  "Joel",
  "Amos",
  "Obad",
  "Jonah",
  "Mic",
  "Nah",
  "Hab",
  "Zeph",
  "Hag",
  "Zech",
  "Mal",
  "Matt",
  "Mark",
  "Luke",
  "John",
  "Acts",
  "Rom",
  "1Cor",
  "2Cor",
  "Gal",
  "Eph",
  "Phil",
  "Col",
  "1Thess",
  "2Thess",
  "1Tim",
  "2Tim",
  "Titus",
  "Phlm",
  "Heb",
  "Jas",
  "1Pet",
  "2Pet",
  "1John",
  "2John",
  "3John",
  "Jude",
  "Rev",
];
export const books = introductions.map((b, i) => ({
  ...b,
  osis: names[i],
  art: art[`../../../../assets/ui/book-covers/${names[i]}.svg`],
}));
const featuredPassages = [
  {
    book: 20,
    chapter: 16,
    section: "Wisdom literature",
    verses: [
      "The preparations of the heart in man, and the answer of the tongue, is from the LORD.",
      "All the ways of a man are clean in his own eyes; but the LORD weigheth the spirits.",
      "Commit thy works unto the LORD, and thy thoughts shall be established.",
      "The LORD hath made all things for himself: yea, even the wicked for the day of evil.",
    ],
  },
  {
    book: 1,
    chapter: 1,
    section: "The beginning",
    verses: [
      "In the beginning God created the heaven and the earth.",
      "And the earth was without form, and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters.",
      "And God said, Let there be light: and there was light.",
    ],
  },
  {
    book: 19,
    chapter: 23,
    section: "Poetry & prayer",
    verses: [
      "The LORD is my shepherd; I shall not want.",
      "He maketh me to lie down in green pastures: he leadeth me beside the still waters.",
      "He restoreth my soul: he leadeth me in the paths of righteousness for his name’s sake.",
    ],
  },
  {
    book: 43,
    chapter: 3,
    section: "The Gospels",
    verses: [
      "There was a man of the Pharisees, named Nicodemus, a ruler of the Jews:",
      "The same came to Jesus by night, and said unto him, Rabbi, we know that thou art a teacher come from God: for no man can do these miracles that thou doest, except God be with him.",
      "Jesus answered and said unto him, Verily, verily, I say unto thee, Except a man be born again, he cannot see the kingdom of God.",
    ],
  },
];
export const passages = [
  ...featuredPassages,
  ...bookOpenings.filter(
    (p) =>
      !featuredPassages.some(
        (f) => f.book === p.book && f.chapter === p.chapter,
      ),
  ),
];
export type Block = {
  native?: import("../native").NativeBlock;
  id: string;
  kind: "thought" | "passage" | "resource" | "word";
  text: string;
  source?: string;
  ref?: string;
};
export type Note = {
  endRef?: number;
  nativeSelection?: import("../native").NativeSelection;
  id: string;
  title: string;
  anchor: string;
  quote: string;
  blocks: Block[];
  pinned: boolean;
  updated: string;
};
export const initialNotes: Note[] = [
  {
    id: "seed-1",
    title: "The work & the weight",
    anchor: "Proverbs 16:3",
    quote: "Commit thy works unto the LORD",
    pinned: true,
    updated: "Today",
    blocks: [
      {
        id: "t1",
        kind: "thought",
        text: "I keep coming back to the difference between doing the work and carrying the outcome. Faithfulness asks me to show up, and trust asks me to let go.",
      },
      {
        id: "p1",
        kind: "passage",
        ref: "Psalms 37:5",
        text: "Commit thy way unto the LORD; trust also in him; and he shall bring it to pass.",
        source: "King James Version",
      },
      {
        id: "t2",
        kind: "thought",
        text: "What would it look like to bring this into the week ahead?",
      },
    ],
  },
  {
    id: "seed-2",
    title: "A little room for stillness",
    anchor: "Psalms 23:2",
    quote: "He leadeth me beside the still waters.",
    pinned: false,
    updated: "Yesterday",
    blocks: [
      {
        id: "t3",
        kind: "thought",
        text: "Rest is part of being led. I don’t have to turn every quiet moment into another task.",
      },
    ],
  },
];
export const samples: Block[] = [
  { id: "thought", kind: "thought", text: "" },
  {
    id: "passage",
    kind: "passage",
    ref: "Psalms 37:5",
    text: "Commit thy way unto the LORD; trust also in him; and he shall bring it to pass.",
    source: "King James Version",
  },
  {
    id: "resource",
    kind: "resource",
    ref: "Genesis · Introduction",
    text: "Genesis is a name borrowed from the Greek. It signifies the original, or generation: fitly is this book so called, for it is a history of originals",
    source: "Matthew Henry · Book introduction",
  },
  {
    id: "word",
    kind: "word",
    ref: "H3068",
    text: "The self-Existent or Eternal; Jehovah, Jewish national name of God.",
    source: "Strong’s Hebrew dictionary · H3068",
  },
];
// Preview data is isolated from installed libraries. A Tauri adapter will implement this boundary.
const key = "fontes-heroui-preview-notes-v1";
export function loadNotes(): Note[] {
  try {
    const saved = JSON.parse(localStorage.getItem(key) || "null");
    if (
      Array.isArray(saved) &&
      saved.every(
        (n) =>
          typeof n.id === "string" &&
          typeof n.title === "string" &&
          Array.isArray(n.blocks),
      )
    )
      return saved;
  } catch {}
  return structuredClone(initialNotes);
}
export function persistNotes(notes: Note[]) {
  localStorage.setItem(key, JSON.stringify(notes));
}
