(function () {
  const PROFILE_STORAGE_KEY = "bloxhub.fflag.profiles";
  const LAST_BACKUP_KEY = "bloxhub.fflag.lastbackup";

  function cloneFlags(value) {
    try {
      return JSON.parse(JSON.stringify(value || {}));
    } catch {
      return {};
    }
  }

  function loadProfiles() {
    if (typeof state === "undefined") return;
    try {
      const raw = localStorage.getItem(PROFILE_STORAGE_KEY);
      state.ffProfiles = Array.isArray(raw ? JSON.parse(raw) : []) ? JSON.parse(raw) : [];
    } catch {
      state.ffProfiles = [];
    }
    if (!Array.isArray(state.ffProfiles)) state.ffProfiles = [];
    state.ffProfiles = state.ffProfiles.filter((p) => p && typeof p.name === "string" && p.flags && typeof p.flags === "object");
  }

  function saveProfiles() {
    if (typeof state === "undefined") return;
    localStorage.setItem(PROFILE_STORAGE_KEY, JSON.stringify(state.ffProfiles || []));
  }

  function saveLastBackup(flags) {
    localStorage.setItem(LAST_BACKUP_KEY, JSON.stringify(cloneFlags(flags || {})));
  }

  function renderProfiles() {
    if (typeof state === "undefined") return;
    const list = document.getElementById("ff-profiles-list");
    if (!list) return;

    list.textContent = "";
    if (!state.ffProfiles || !state.ffProfiles.length) {
      const empty = document.createElement("div");
      empty.className = "subtle";
      empty.textContent = "Aucun profil enregistré.";
      list.appendChild(empty);
      return;
    }

    for (const profile of state.ffProfiles) {
      const item = document.createElement("div");
      item.className = "profile-item";

      const head = document.createElement("div");
      head.className = "profile-item-head";

      const name = document.createElement("div");
      name.className = "profile-item-name";
      name.textContent = profile.name;

      const actions = document.createElement("div");
      actions.className = "profile-item-actions";

      const applyBtn = document.createElement("button");
      applyBtn.className = "mini-btn";
      applyBtn.textContent = "Appliquer";
      applyBtn.addEventListener("click", () => {
        state.pendingFlags = cloneFlags(profile.flags);
        if (typeof window.renderFfAll === "function") {
          window.renderFfAll();
        }
        toast("Profil chargé : " + profile.name, "ok");
      });

      const deleteBtn = document.createElement("button");
      deleteBtn.className = "mini-btn danger";
      deleteBtn.textContent = "Suppr.";
      deleteBtn.addEventListener("click", () => {
        state.ffProfiles = (state.ffProfiles || []).filter((p) => p.name !== profile.name);
        saveProfiles();
        renderProfiles();
        toast("Profil supprimé : " + profile.name, "ok");
      });

      actions.appendChild(applyBtn);
      actions.appendChild(deleteBtn);
      head.appendChild(name);
      head.appendChild(actions);

      const meta = document.createElement("div");
      meta.className = "mini-meta";
      meta.textContent = Object.keys(profile.flags || {}).length + " flag(s)";

      item.appendChild(head);
      item.appendChild(meta);
      list.appendChild(item);
    }
  }

  function restoreLastBackup() {
    if (typeof state === "undefined") return;
    try {
      const raw = localStorage.getItem(LAST_BACKUP_KEY);
      const backup = raw ? JSON.parse(raw) : null;
      if (!backup || typeof backup !== "object") {
        toast("Aucun backup disponible pour le moment.", "err");
        return;
      }
      state.pendingFlags = cloneFlags(backup);
      if (typeof window.renderFfAll === "function") {
        window.renderFfAll();
      }
      toast("Backup restauré.", "ok");
    } catch {
      toast("Le backup est invalide.", "err");
    }
  }

  function saveCurrentProfile() {
    if (typeof state === "undefined") return;
    const input = document.getElementById("ff-profile-name");
    const name = (input && input.value ? input.value : "").trim();
    if (!name) {
      toast("Entre un nom pour le profil.", "err");
      return;
    }

    const flags = cloneFlags(state.pendingFlags || {});
    const existing = (state.ffProfiles || []).find((p) => p.name.toLowerCase() === name.toLowerCase());
    if (existing) {
      existing.flags = flags;
      toast("Profil mis à jour : " + name, "ok");
    } else {
      state.ffProfiles.push({ name, flags });
      toast("Profil enregistré : " + name, "ok");
    }

    saveProfiles();
    renderProfiles();
    if (input) input.value = "";
  }

  function installProfileBindings() {
    if (typeof state === "undefined") return;

    state.ffProfiles = state.ffProfiles || [];
    state.ffLastBackup = state.ffLastBackup || null;
    loadProfiles();

    const saveBtn = document.getElementById("ff-save-profile");
    if (saveBtn) {
      saveBtn.addEventListener("click", saveCurrentProfile);
    }

    const restoreBtn = document.getElementById("ff-restore-last");
    if (restoreBtn) {
      restoreBtn.addEventListener("click", restoreLastBackup);
    }

    const originalRenderFfAll = window.renderFfAll;
    if (typeof originalRenderFfAll === "function") {
      window.renderFfAll = function () {
        originalRenderFfAll();
        renderProfiles();
      };
    } else {
      window.renderFfAll = function () {
        renderProfiles();
      };
    }

    renderProfiles();

    const applyBtn = document.getElementById("ff-apply");
    if (applyBtn) {
      const originalListener = applyBtn.onclick;
      applyBtn.onclick = function (event) {
        if (typeof originalListener === "function") {
          originalListener.call(this, event);
        }
        saveLastBackup(state.currentFlags || {});
      };
    }
  }

  document.addEventListener("DOMContentLoaded", function () {
    if (typeof state !== "undefined") {
      installProfileBindings();
    }
  });
})();
