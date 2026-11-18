import { useLayoutEffect, type ReactNode } from "react";
import { Modal } from "@heroui/react";
export function Sheet({
  title,
  open,
  onClose,
  children,
  footer,
  edgeToEdge = false,
  closeLabel = "Close",
  className,
  eyebrow,
  description,
  topLine,
}: {
  title: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  edgeToEdge?: boolean;
  closeLabel?: string;
  className?: string;
  eyebrow?: string;
  description?: ReactNode;
  topLine?: ReactNode;
}) {
  useLayoutEffect(() => {
    // Android draws WebView selection handles above HTML overlays. Callers
    // retain the quote/spans in React state; dismiss only the live DOM range.
    if (!open) return;
    let active = true;
    const dismissSelection = () => {
      if (active) window.getSelection()?.removeAllRanges();
    };
    dismissSelection();
    // React/Heroui focus moves can reinstate a DOM range after the sheet opens.
    queueMicrotask(dismissSelection);
    const frame = requestAnimationFrame(dismissSelection);
    return () => {
      active = false;
      cancelAnimationFrame(frame);
    };
  }, [open]);
  return (
    <Modal
      isOpen={open}
      onOpenChange={(value) => {
        if (!value) onClose();
      }}
    >
      <Modal.Backdrop
        UNSTABLE_portalContainer={
          document.getElementById("overlay-root") || undefined
        }
      >
        <Modal.Container
          placement="bottom"
          size="md"
          className={
            [edgeToEdge && "edge-sheet", className].filter(Boolean).join(" ") ||
            undefined
          }
        >
          <Modal.Dialog>
            {eyebrow && <div className="sheet-grabber" aria-hidden="true" />}
            <Modal.CloseTrigger aria-label={closeLabel} />
            {topLine && <div className="sheet-topline">{topLine}</div>}
            <Modal.Header>
              {eyebrow && <div className="sheet-eyebrow">{eyebrow}</div>}
              <Modal.Heading>{title}</Modal.Heading>
              {description && (
                <div className="sheet-description">{description}</div>
              )}
            </Modal.Header>
            <Modal.Body>{children}</Modal.Body>
            {footer && <Modal.Footer>{footer}</Modal.Footer>}
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
