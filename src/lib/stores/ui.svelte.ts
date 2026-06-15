import type { View } from "$lib/types";

class UiStore {
  currentView = $state<View>("main");
  searchQuery = $state("");
  showEditor = $state(false);
  showGenerator = $state(false);
  mobileMenuOpen = $state(false);
  notification = $state<{ message: string; type: "success" | "error" } | null>(null);

  navigate(view: View) {
    this.currentView = view;
    this.mobileMenuOpen = false;
  }

  notify(message: string, type: "success" | "error" = "success") {
    this.notification = { message, type };
    setTimeout(() => {
      this.notification = null;
    }, 3000);
  }
}

export const ui = new UiStore();
