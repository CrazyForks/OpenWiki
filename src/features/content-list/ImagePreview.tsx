import { useEffect } from "react";
import { createPortal } from "react-dom";

interface ImagePreviewProps {
  src: string;
  onClose: () => void;
}

export function ImagePreview({ src, onClose }: ImagePreviewProps) {
  // Close on Escape key
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      onClick={onClose}
    >
      {/* Close button */}
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-white/80 hover:text-white text-2xl
                   w-10 h-10 flex items-center justify-center rounded-full
                   bg-black/30 hover:bg-black/50 transition-colors"
      >
        ✕
      </button>

      {/* Image */}
      <img
        src={src}
        alt="Preview"
        className="max-w-[90vw] max-h-[90vh] object-contain rounded-lg shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      />
    </div>,
    document.body
  );
}
