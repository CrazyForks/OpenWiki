import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTranslation } from "react-i18next";
import { Sparkles, X, Loader2 } from "lucide-react";
import {
  dismissUpdateVersion,
  type UpdateInfo,
} from "../../services/updateService";

type InstallState = "idle" | "downloading" | "installing" | "failed";

/**
 * Top-of-main-window banner announcing a newer OpenWiki version.
 *
 * Notification source is the existing GitHub-Releases polling backend
 * (`src-tauri/src/update/mod.rs`). When the user clicks "立即升级", we
 * hand off to `tauri-plugin-updater` which downloads the signed bundle,
 * verifies it against the embedded public key, installs it, and relaunches
 * — same one-click flow as Slack / Linear / VS Code.
 */
export function UpdateBanner() {
  const { t } = useTranslation("update");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [installState, setInstallState] = useState<InstallState>("idle");
  const [errorMsg, setErrorMsg] = useState<string>("");
  const [fallbackToDownload, setFallbackToDownload] = useState(false);

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

  if (!info) return null;

  const handleInstall = async () => {
    if (fallbackToDownload) {
      await handleViewNotes();
      return;
    }

    setInstallState("downloading");
    setErrorMsg("");
    setFallbackToDownload(false);
    try {
      const update = await check();
      if (!update) {
        setInstallState("failed");
        setErrorMsg("No update found");
        setFallbackToDownload(true);
        await handleViewNotes();
        return;
      }
      await update.downloadAndInstall((event) => {
        if (event.event === "Finished") {
          setInstallState("installing");
        }
      });
      await relaunch();
    } catch (err) {
      console.error("[update] downloadAndInstall failed:", err);
      setInstallState("failed");
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setFallbackToDownload(true);

      const message = err instanceof Error ? err.message : String(err);
      if (
        message.includes("release JSON") ||
        message.includes("No update found")
      ) {
        await handleViewNotes();
      }
    }
  };

  const handleViewNotes = async () => {
    try {
      await openExternal(info.url);
    } catch (err) {
      console.error("[update] failed to open release page:", err);
    }
  };

  const handleLater = async () => {
    try {
      await dismissUpdateVersion(info.version);
    } catch (err) {
      console.error("[update] failed to dismiss version:", err);
    }
    setInfo(null);
  };

  const installLabel =
    fallbackToDownload
      ? t("banner.downloadFallback")
      : installState === "downloading"
      ? t("banner.downloading")
      : installState === "installing"
      ? t("banner.installing")
      : t("banner.install");

  const isWorking =
    installState === "downloading" || installState === "installing";

  return (
    <div
      role="status"
      aria-live="polite"
      className="sticky top-[40px] z-[9] border-b border-orange-200/60 dark:border-orange-500/20
                 bg-orange-50/95 dark:bg-orange-500/[0.08] backdrop-blur-xl
                 animate-in fade-in slide-in-from-top-2 duration-300"
    >
      <div className="flex items-center gap-3 px-4 py-2.5 max-w-full">
        <Sparkles className="w-4 h-4 text-orange-500 flex-shrink-0" />

        <div className="flex-1 min-w-0">
          <div className="text-[13px] font-medium text-gray-900 dark:text-orange-50 truncate">
            {t("banner.title", { version: info.version })}
          </div>
          <div className="text-[11px] text-gray-500 dark:text-orange-200/70 truncate">
            {installState === "failed"
              ? fallbackToDownload
                ? t("banner.failedFallback")
                : t("banner.failed", { error: errorMsg })
              : t("banner.subtitle", { current: info.current_version })}
          </div>
        </div>

        <button
          onClick={handleInstall}
          disabled={isWorking}
          className="flex-shrink-0 px-3 py-1 text-[12px] font-medium rounded-md
                     bg-orange-500 text-white hover:bg-orange-600
                     disabled:opacity-70 disabled:cursor-wait
                     transition-colors shadow-sm
                     flex items-center gap-1.5"
        >
          {isWorking && <Loader2 className="w-3 h-3 animate-spin" />}
          {installLabel}
        </button>

        <button
          onClick={handleViewNotes}
          className="flex-shrink-0 px-3 py-1 text-[12px] font-medium rounded-md
                     text-orange-700 dark:text-orange-200 hover:bg-orange-100/60
                     dark:hover:bg-orange-500/[0.15] transition-colors"
        >
          {t("banner.view")}
        </button>

        <button
          onClick={handleLater}
          aria-label={t("banner.close")}
          className="flex-shrink-0 p-1 rounded text-gray-400 hover:text-gray-700
                     dark:text-orange-300/60 dark:hover:text-orange-200
                     hover:bg-orange-100/40 dark:hover:bg-orange-500/10 transition-colors"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  );
}
