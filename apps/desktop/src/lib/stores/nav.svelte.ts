import { inApp } from "../api.ts";
import { NAV, type ViewId } from "./nav.ts";

const KEY = "macmetry.view";

class NavStore {
  view = $state<ViewId>("overview");

  constructor() {
    try {
      const saved = localStorage.getItem(KEY) as ViewId | null;
      if (saved && NAV.some((n) => n.id === saved)) this.view = saved;
    } catch { /* storage can be unavailable */ }
    // In a plain browser `?view=` picks the view, for design work and screenshots.
    const asked = inApp ? null : (new URLSearchParams(location.search).get("view") as ViewId | null);
    if (asked && NAV.some((n) => n.id === asked)) this.view = asked;
  }

  go(id: ViewId) {
    this.view = id;
    try { localStorage.setItem(KEY, id); } catch { /* storage can be unavailable */ }
  }
}

export const nav = new NavStore();
