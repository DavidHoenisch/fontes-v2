import { useState } from "react";
import { Button } from "@heroui/react";
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { Download, RefreshCw, Check, BookOpen } from "lucide-react";
import { Sheet } from "./Sheet";
export type UpdateInfo = {
  installed_sequence: number | null;
  available_sequence: number;
  update_available: boolean;
  download_bytes: number;
  installed_bytes: number;
  temporary_bytes: number;
  manifest_sha256: string;
  release_url: string;
};
type Progress = { message: string; completed: number; total: number | null };
const size = (bytes: number) => `${Math.ceil(bytes / 1024 / 1024)} MB`;
export function ResourceUpdates({ reload }: { reload?: () => Promise<void> }) {
  const [open, setOpen] = useState(false);
  const [phase, setPhase] = useState<"idle" | "checking" | "offer" | "updating" | "activated" | "done">("idle");
  const [offer, setOffer] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [cancelling, setCancelling] = useState(false);
  const busy = phase === "checking" || phase === "updating";
  async function check() {
    setPhase("checking"); setError(""); setOffer(null);
    try {
      if (!isTauri()) throw Error("Resource updates are available in the installed Fontes app. This browser uses a sample library.");
      const result = await invoke<UpdateInfo>("library_check_update");
      setOffer(result); setPhase("offer");
    } catch (e) { setError(String(e)); setPhase("idle"); }
  }
  async function reopen() {
    try { await reload?.(); setPhase("done"); setError(""); }
    catch (e) { setPhase("activated"); setError(`The update is installed. We couldn’t reopen it yet: ${String(e)}. Retry opening, or restart Fontes.`); }
  }
  async function update() {
    if (!offer) return;
    setPhase("updating"); setError(""); setCancelling(false);
    setProgress({ message: "Preparing your update…", completed: 0, total: null });
    const channel = new Channel<Progress>();
    channel.onmessage = setProgress;
    try {
      await invoke("library_update", { manifestSha256: offer.manifest_sha256, progress: channel });
      setPhase("activated");
      await reopen();
    } catch (e) { setError(String(e)); setPhase("idle"); setOffer(null); }
    finally { channel.onmessage = () => {}; setCancelling(false); }
  }
  async function cancel() {
    setCancelling(true);
    try { await invoke("library_cancel_update"); }
    catch (e) { setError(String(e)); setCancelling(false); }
  }
  return <>
    <button className="resource-update-entry" onClick={() => setOpen(true)}>
      <span className="resource-update-icon"><RefreshCw size={18} /></span>
      <span><strong>Free resources</strong><small>{phase === "done" ? "Your library is up to date" : "Check for resource updates"}</small></span>
      <span aria-hidden="true">›</span>
    </button>
    <Sheet title="Keep your library growing" open={open} onClose={() => { if (!busy && phase !== "activated") setOpen(false); }} footer={
      phase === "updating" ? <Button variant="secondary" isDisabled={cancelling} onPress={() => void cancel()}>{cancelling ? "Cancelling…" : "Cancel update"}</Button> :
      phase === "activated" ? <Button onPress={() => void reopen()}>Open updated library</Button> :
      phase === "done" || (phase === "offer" && !offer?.update_available) ? <Button onPress={() => setOpen(false)}>Done</Button> :
      phase === "offer" && offer?.update_available ? <Button onPress={() => void update()}><Download size={18} /> Download update · {size(offer.download_bytes)}</Button> :
      <Button isDisabled={busy} onPress={() => void check()}><RefreshCw size={18} />{phase === "checking" ? "Checking…" : "Check for updates"}</Button>
    }>
      <div className="resource-update-summary"><BookOpen size={24} /><p>Bibles and study tools, updated together.</p></div>
      <p className="muted">Your notes, highlights, bookmarks, and reading place stay with you.</p>
      {phase === "checking" && <p role="status">Checking for a verified resource release…</p>}
      {phase === "offer" && offer && (offer.update_available ? <div className="resource-update-offer">
        <h3>A fresh edition is ready</h3>
        <p>{offer.installed_sequence ? `Version ${offer.installed_sequence} → ${offer.available_sequence}` : `Resource version ${offer.available_sequence}`}</p>
        <dl><div><dt>Download</dt><dd>{size(offer.download_bytes)}</dd></div><div><dt>Space for installation</dt><dd>About {size(offer.temporary_bytes)}</dd></div></dl>
        <p className="small muted">Includes the complete free collection. Keep Fontes open while it downloads and verifies the update.</p>
        <a href={offer.release_url} target="_blank" rel="noreferrer">What’s in this release ↗</a>
      </div> : <p className="resource-update-success" role="status"><Check size={20} /> You have the latest resources · version {offer.available_sequence}</p>)}
      {phase === "updating" && progress && <div className="resource-update-progress" role="status">
        <p>{cancelling ? "Finishing cancellation…" : progress.message}</p>
        <progress aria-label="Resource update progress" value={progress.total ? progress.completed : undefined} max={progress.total || 1} />
        {progress.total && <p className="small muted">{Math.min(100, Math.floor(progress.completed / progress.total * 100))}%</p>}
      </div>}
      {phase === "done" && <p className="resource-update-success" role="status"><Check size={20} /> Your updated library is ready.</p>}
      {error && <p role="alert" className="resource-update-error">{error}</p>}
      {phase === "idle" && <p className="small muted">Checking downloads only release information. You choose when to download the resources. Your current library remains available if a download fails.</p>}
    </Sheet>
  </>;
}
