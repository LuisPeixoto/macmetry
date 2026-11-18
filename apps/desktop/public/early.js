// Runs before the page paints, so the theme is right from the first frame. It is a separate file because the app's
// content security policy does not allow inline scripts.
try {
  var root = document.documentElement;
  var inApp = "__TAURI_INTERNALS__" in window;
  var s = localStorage.getItem("macmetry.theme") || "system";
  // In a plain browser (design work and screenshots) `?theme=dark` or `?theme=light` overrides the setting.
  if (!inApp) { var q = new URLSearchParams(location.search).get("theme"); if (q === "dark" || q === "light") s = q; }
  var dark = s === "dark" || (s === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  root.dataset.theme = dark ? "dark" : "light";
  if (!inApp) root.classList.add("no-glass");
} catch (e) {}
