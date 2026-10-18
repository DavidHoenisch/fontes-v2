import type { ReactNode } from "react";
import { Modal } from "@heroui/react";
export function Sheet({
  title,
  open,
  onClose,
  children,
  footer,
}: {
  title: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}) {
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
        <Modal.Container placement="bottom" size="md">
          <Modal.Dialog>
            <Modal.CloseTrigger />
            <Modal.Header>
              <Modal.Heading>{title}</Modal.Heading>
            </Modal.Header>
            <Modal.Body>{children}</Modal.Body>
            {footer && <Modal.Footer>{footer}</Modal.Footer>}
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
