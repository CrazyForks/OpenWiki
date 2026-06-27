export const SECTION_THEME: Record<string, {
  iconPath: string;
  keyword: string;
  accent: string;
  accentText: string;
}> = {
  key_insight: {
    iconPath: "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z",
    keyword: "ALERT",
    accent: "bg-red-50 dark:bg-red-500/15",
    accentText: "text-red-500 dark:text-red-400",
  },
  highlight: {
    iconPath: "M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z",
    keyword: "STAR",
    accent: "bg-amber-50 dark:bg-amber-500/15",
    accentText: "text-amber-500 dark:text-amber-400",
  },
  trend: {
    iconPath: "M2.25 18L9 11.25l4.306 4.307a11.95 11.95 0 015.814-5.519l2.74-1.22m0 0l-5.94-2.28m5.94 2.28l-2.28 5.941",
    keyword: "TREND",
    accent: "bg-orange-500/10 dark:bg-orange-500/20",
    accentText: "text-orange-500 dark:text-orange-400",
  },
  routine: {
    iconPath: "M8.25 6.75h12M8.25 12h12m-12 5.25h12M3.75 6.75h.007v.008H3.75V6.75zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z",
    keyword: "NOTE",
    accent: "bg-gray-50 dark:bg-slate-500/15",
    accentText: "text-gray-400 dark:text-slate-400",
  },
  recommendation: {
    iconPath: "M12 18v-5.25m0 0a6.01 6.01 0 001.5-.189m-1.5.189a6.01 6.01 0 01-1.5-.189m3.75 7.478a12.06 12.06 0 01-4.5 0m3.75 2.383a14.406 14.406 0 01-3 0M14.25 18v-.192c0-.983.658-1.823 1.508-2.316a7.5 7.5 0 10-7.517 0c.85.493 1.509 1.333 1.509 2.316V18",
    keyword: "IDEA",
    accent: "bg-emerald-50 dark:bg-emerald-500/15",
    accentText: "text-emerald-500 dark:text-emerald-400",
  },
  topic: {
    iconPath: "M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z",
    keyword: "TAG",
    accent: "bg-gray-50 dark:bg-slate-500/15",
    accentText: "text-gray-400 dark:text-slate-400",
  },
};
