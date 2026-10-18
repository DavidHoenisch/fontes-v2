import { useEffect, useRef, useState } from "react";
import { Button, Card } from "@heroui/react";
import { ArrowLeft, ChevronRight, BookOpen } from "lucide-react";
import { Sheet } from "./Sheet";
import type { Live } from "../native";
type Target = {
  start_ref: number;
  end_ref: number;
  label: string;
  preview: string | null;
  verses?: Verse[];
};
type Verse = { ref_id: number; label: string; text: string };
export function CrossReferences({
  reference,
  onClose,
  live,
}: {
  reference: { id: number; label: string } | null;
  onClose: () => void;
  live?: Live;
}) {
  const [targets, setTargets] = useState<Target[]>([]),
    [active, setActive] = useState<Target | null>(null),
    [verses, setVerses] = useState<Verse[]>([]),
    [loading, setLoading] = useState(false),
    [error, setError] = useState(""),
    [more, setMore] = useState(false);
  const request = useRef(0);
  useEffect(() => {
    const generation = ++request.current;
    setActive(null);
    setVerses([]);
    setError("");
    setTargets([]);
    if (!reference) return;
    setLoading(true);
    void (async () => {
      try {
        let refs: Target[];
        if (live) {
          const r = await live.send({
            command: "study.context",
            params: { ref_id: reference.id },
          });
          refs = r.data.cross_references as Target[];
        } else {
          const data = (await import("../data/cross-references.json"))
            .default as Record<string, Target[]>;
          refs = data[String(reference.id)] || [];
        }
        if (generation === request.current) setTargets(refs);
      } catch (e) {
        if (generation === request.current) setError(String(e));
      } finally {
        if (generation === request.current) setLoading(false);
      }
    })();
    return () => {
      request.current++;
    };
  }, [reference?.id, live?.state.session.translation]);
  async function preview(target: Target) {
    const generation = ++request.current;
    setActive(target);
    setLoading(true);
    setError("");
    setVerses([]);
    setMore(false);
    try {
      if (live) {
        const r = await live.send({
          command: "reading.preview",
          params: { start_ref: target.start_ref, end_ref: target.end_ref },
        });
        if (generation === request.current) {
          setVerses(r.data.verses as Verse[]);
          setMore(Boolean(r.data.has_more));
        }
      } else if (generation === request.current) setVerses(target.verses || []);
    } catch (e) {
      if (generation === request.current) setError(String(e));
    } finally {
      if (generation === request.current) setLoading(false);
    }
  }
  return (
    <Sheet
      title={active ? active.label : `Cross-references`}
      open={reference !== null}
      onClose={onClose}
    >
      {active ? (
        <Button
          variant="ghost"
          size="sm"
          className="mb-4"
          onPress={() => {
            request.current++;
            setActive(null);
            setLoading(false);
            setError("");
          }}
        >
          <ArrowLeft size={16} /> Back to {reference?.label}
        </Button>
      ) : (
        <div className="crossref-heading">
          <BookOpen size={20} />
          <div>
            <strong>{reference?.label}</strong>
            <p className="muted small">
              Treasury of Scripture Knowledge ·{" "}
              {live
                ? live.state.translations.find(
                    (t) => t.id === live.state.session.translation,
                  )?.abbr
                : "KJV"}
            </p>
          </div>
        </div>
      )}
      {loading && (
        <p role="status" className="muted">
          Loading {active ? "passage" : "cross-references"}…
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {!loading &&
        !error &&
        (active ? (
          <div className="crossref-passage">
            {verses.length ? (
              verses.map((v) => (
                <div key={v.ref_id}>
                  <small className="muted">{v.label}</small>
                  <p className="scripture">{v.text}</p>
                </div>
              ))
            ) : (
              <p className="muted">
                This passage is not available in the active translation.
              </p>
            )}
            {more && (
              <p className="muted small">
                Showing the first 100 verse segments in this range.
              </p>
            )}
          </div>
        ) : (
          <div className="crossref-list">
            {targets.map((t) => (
              <Button
                key={`${t.start_ref}-${t.end_ref}`}
                variant="ghost"
                fullWidth
                onPress={() => void preview(t)}
              >
                <span>
                  <strong>{t.label}</strong>
                  {t.preview && <small>{t.preview}</small>}
                </span>
                <ChevronRight size={18} />
              </Button>
            ))}
            {!targets.length && (
              <Card>
                <Card.Content>
                  <p className="muted">
                    {live
                      ? "No cross-references are supplied for this verse in your installed library. Older Reading packages need the updated TSK-enabled data release."
                      : "No TSK sample for this verse in the browser preview. Try Proverbs 16:1–4."}
                  </p>
                </Card.Content>
              </Card>
            )}
          </div>
        ))}
    </Sheet>
  );
}
