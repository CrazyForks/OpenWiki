import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Download, Loader2, X } from "lucide-react";
import { motion } from "framer-motion";
import { type UpdateInfo } from "../../services/updateService";

type PrepareState = "idle" | "preparing" | "ready" | "installing" | "failed";
type FailureStage = "download" | "install" | null;

/**
 * Silent update preparer + restart confirmation dialog.
 *
 * Notification source is the existing GitHub-Releases polling backend
 * (`src-tauri/src/update/mod.rs`). Once a newer version is detected, we
 * quietly download it through `tauri-plugin-updater`; only after the update
 * package is ready do we ask the user whether to install and relaunch.
 */
export function UpdateBanner() {
  const { t } = useTranslation("update");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [downloadedUpdate, setDownloadedUpdate] = useState<Update | null>(null);
  const [prepareState, setPrepareState] = useState<PrepareState>("idle");
  const [failureStage, setFailureStage] = useState<FailureStage>(null);
  const [errorMsg, setErrorMsg] = useState<string>("");
  const [downloadAttempt, setDownloadAttempt] = useState(0);
  const preparingVersionRef = useRef<string | null>(null);

  useEffect(() => {
    const unlisten = listen<UpdateInfo>("update-available", (event) => {
      setInfo(event.payload);
    });

    const manualHandler = (e: Event) => {
      const ce = e as CustomEvent<UpdateInfo>;
      if (ce.detail) setInfo(ce.detail);
    };
    window.addEventListener("update-available-manual", manualHandler);

    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("update-available-manual", manualHandler);
    };
  }, []);

  useEffect(() => {
    if (!info || preparingVersionRef.current === info.version) {
      return;
    }

    let cancelled = false;
    preparingVersionRef.current = info.version;
    setPrepareState("preparing");
    setFailureStage(null);
    setErrorMsg("");

    const prepareUpdate = async () => {
      const update = await check();
      if (!update) {
        throw new Error("Update package is not available");
      }

      if (cancelled) {
        await update.close().catch((err) => {
          console.error("[update] failed to close cancelled update:", err);
        });
        return;
      }

      try {
        await update.download();
      } catch (err) {
        await update.close().catch((closeErr) => {
          console.error("[update] failed to close unsuccessful download:", closeErr);
        });
        throw err;
      }

      if (cancelled) {
        await update.close().catch((err) => {
          console.error("[update] failed to close cancelled download:", err);
        });
        return;
      }

      setDownloadedUpdate(update);
      setFailureStage(null);
      setPrepareState("ready");
    };

    prepareUpdate().catch((err) => {
      console.error("[update] background download failed:", err);
      if (!cancelled) {
        preparingVersionRef.current = null;
        setFailureStage("download");
        setPrepareState("failed");
        setErrorMsg(err instanceof Error ? err.message : String(err));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [info, downloadAttempt]);

  if (!info || prepareState === "idle" || prepareState === "preparing") {
    return null;
  }

  const handleInstall = async () => {
    setPrepareState("installing");
    setFailureStage(null);
    setErrorMsg("");
    try {
      if (!downloadedUpdate) {
        throw new Error("Update package is not ready");
      }
      await downloadedUpdate.install();
      await relaunch();
    } catch (err) {
      console.error("[update] install failed:", err);
      setFailureStage("install");
      setPrepareState("failed");
      setErrorMsg(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRetryDownload = async () => {
    if (downloadedUpdate) {
      try {
        await downloadedUpdate.close();
      } catch (err) {
        console.error("[update] failed to close update before retry:", err);
      }
    }
    setDownloadedUpdate(null);
    preparingVersionRef.current = null;
    setFailureStage(null);
    setPrepareState("preparing");
    setErrorMsg("");
    setDownloadAttempt((attempt) => attempt + 1);
  };

  const handleClose = () => {
    if (prepareState === "installing") {
      return;
    }

    if (downloadedUpdate) {
      downloadedUpdate.close().catch((err) => {
        console.error("[update] failed to close downloaded update:", err);
      });
    }
    setDownloadedUpdate(null);
    preparingVersionRef.current = null;
    setPrepareState("idle");
    setFailureStage(null);
    setInfo(null);
    setErrorMsg("");
  };

  const handleViewNotes = async () => {
    try {
      await openExternal(info.url);
    } catch (err) {
      console.error("[update] failed to open release page:", err);
    }
  };

  const title =
    prepareState === "failed"
      ? t("dialog.failedTitle")
      : t("dialog.title", { version: info.version });

  const description =
    prepareState === "failed"
      ? t(failureStage === "install" ? "dialog.installFailedBody" : "dialog.downloadFailedBody", { error: errorMsg })
      : t("dialog.body", { version: info.version });

  const primaryLabel =
    prepareState === "installing"
      ? t("dialog.installing")
      : prepareState === "failed"
      ? t("dialog.retryDownload")
      : t("dialog.install");

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      className="fixed bottom-4 left-4 right-4 z-[100] flex justify-center
                 pointer-events-none
                 sm:bottom-5 sm:left-auto sm:right-5 sm:justify-end"
      role="dialog"
      aria-live="polite"
      aria-labelledby="update-ready-title"
      aria-describedby="update-ready-description"
    >
      <div
        className="relative w-full max-w-[420px] rounded-2xl pointer-events-auto
                   border border-stone-200/70 dark:border-white/[0.08]
                   bg-white text-stone-900 dark:bg-stone-900 dark:text-stone-50
                   shadow-[0_24px_70px_rgba(28,25,23,0.24)]
                   p-7"
      >
        <button
          onClick={handleClose}
          disabled={prepareState === "installing"}
          aria-label={t("dialog.close")}
          className="absolute right-4 top-4 rounded-lg p-1.5 text-stone-400
                     transition-colors hover:bg-stone-100 hover:text-stone-700
                     disabled:cursor-not-allowed disabled:opacity-40
                     dark:text-stone-500 dark:hover:bg-white/[0.06] dark:hover:text-stone-200"
        >
          <X className="h-4 w-4" />
        </button>

        <div
          className="mb-5 flex h-12 w-12 items-center justify-center rounded-xl
                     bg-orange-50 text-orange-600 dark:bg-orange-500/15 dark:text-orange-300"
        >
          {prepareState === "failed" ? (
            <Download className="h-6 w-6" strokeWidth={2.2} />
          ) : (
            <CheckCircle2 className="h-6 w-6" strokeWidth={2.2} />
          )}
        </div>

        <h2
          id="update-ready-title"
          className="mb-2 text-xl font-semibold tracking-normal text-stone-900 dark:text-stone-50"
        >
          {title}
        </h2>

        <p
          id="update-ready-description"
          className="mb-6 text-sm leading-6 text-stone-600 dark:text-stone-300"
        >
          {description}
        </p>

        <div className="flex gap-2.5">
          <button
            onClick={handleClose}
            disabled={prepareState === "installing"}
            className="flex-1 rounded-lg border border-stone-200 bg-white py-2.5
                       text-sm font-medium text-stone-600 transition-colors
                       hover:bg-stone-50 disabled:cursor-not-allowed disabled:opacity-50
                       dark:border-white/[0.08] dark:bg-white/[0.03]
                       dark:text-stone-300 dark:hover:bg-white/[0.06]"
          >
            {t("dialog.later")}
          </button>

          <button
            onClick={prepareState === "failed" ? handleRetryDownload : handleInstall}
            disabled={prepareState === "installing"}
            className="flex-1 rounded-lg bg-orange-500 py-2.5 text-sm font-semibold
                       text-white transition-colors hover:bg-orange-600
                       disabled:cursor-wait disabled:opacity-75
                       flex items-center justify-center gap-2"
          >
            {prepareState === "installing" && (
              <Loader2 className="h-4 w-4 animate-spin" />
            )}
            {primaryLabel}
          </button>
        </div>

        <button
          onClick={handleViewNotes}
          disabled={prepareState === "installing"}
          className="mt-3 w-full rounded-lg py-2 text-xs font-medium
                     text-stone-500 transition-colors hover:bg-stone-50 hover:text-orange-600
                     disabled:cursor-not-allowed disabled:opacity-50
                     dark:text-stone-400 dark:hover:bg-white/[0.04] dark:hover:text-orange-300"
        >
          {prepareState === "failed" ? t("dialog.downloadFallback") : t("dialog.view")}
        </button>
      </div>
    </motion.div>
  );
}
