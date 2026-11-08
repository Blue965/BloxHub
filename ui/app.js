/* ═══════════════════════════════════════════════════════════════
   BloxHub — logique de l'interface
   ═══════════════════════════════════════════════════════════════ */
"use strict";

/* ───────────────────────── Aides ───────────────────────── */

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));
const t = (key, vars) => window.BloxHubT(key, vars);

function el(tag, cls, text) {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

function fmtTime(ts) {
  try {
    return new Date(ts * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

function fmtDay(ts) {
  try {
    return new Date(ts * 1000).toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" });
  } catch {
    return "";
  }
}

function truncateMiddle(str, max) {
  if (!str || str.length <= max) return str || "";
  const half = Math.floor((max - 3) / 2);
  return str.slice(0, half) + "…" + str.slice(str.length - half);
}

function avatarEl(name, color, cls) {
  const div = el("div", cls || "msg-avatar");
  div.style.background = color || "#4b5563";
  div.textContent = (name || "?").charAt(0).toUpperCase();
  return div;
}

/* ─────────────────── Couche API (Tauri / démo) ─────────────────── */

const HAS_TAURI = typeof window.__TAURI__ !== "undefined" && !!window.__TAURI__.core;

async function mockInvoke(cmd, args) {
  await new Promise((r) => setTimeout(r, 120));
  switch (cmd) {
    case "get_settings":
      return {
        language: window.BloxHubLang || "fr",
        chat_server_url: "ws://127.0.0.1:8787",
        chat_username: "Demo",
        auto_connect_chat: false,
        roblox_custom_path: null,
      };
    case "save_settings":
      return null;
    case "detect_roblox":
      return { platform: "windows", installed: false, player_path: null, settings_file: null, version_dir: null };
    case "roblox_status":
      return { installed: false, player_path: null, version_dir: null, running: false };
    case "get_flags":
      return { file: "C:\\…\\ClientAppSettings.json (démo)", flags: {} };
    case "apply_flags":
      return { applied: 0, previous: 0, file: "démo", backup: true };
    case "clear_flags":
      return { applied: 0, previous: 0, file: "démo" };
    case "export_flags":
      return { path: "démo", count: 0 };
    case "import_flags":
      return { imported: 0, total: 0 };
    case "flag_catalog":
      return [
        { key: "DFIntTaskSchedulerTargetFps", value: 9999, vtype: "int", category: "performances", desc_fr: "FPS cible du moteur (démo).", desc_en: "Engine target FPS (demo)." },
        { key: "DFFlagDebugRenderForceTechnologyVoxel", value: true, vtype: "bool", category: "graphismes", desc_fr: "Force l'éclairage Voxel (démo).", desc_en: "Forces Voxel lighting (demo)." },
        { key: "DFIntConnectionMTUSize", value: 1400, vtype: "int", category: "reseau", desc_fr: "Taille MTU (démo).", desc_en: "MTU size (demo)." },
        { key: "FFlagDebugDisableTelemetryEventIngest", value: true, vtype: "bool", category: "telemetrie", desc_fr: "Coupe la télémétrie (démo).", desc_en: "Cuts telemetry (demo)." },
      ];
    case "flag_presets":
      return [
        { id: "fps_max", name_fr: "FPS Maximum", name_en: "Max FPS", desc_fr: "Preset de démo.", desc_en: "Demo preset.", flags: { DFIntTaskSchedulerTargetFps: 9999, FIntRenderShadowIntensity: 0 } },
        { id: "latence", name_fr: "Latence réduite", name_en: "Low latency", desc_fr: "Preset de démo.", desc_en: "Demo preset.", flags: { DFIntConnectionMTUSize: 1400 } },
      ];
    case "launch_roblox":
      return "démo";
    case "roblox_profile":
      throw new Error("Mode démo : ouvre BloxHub via Tauri pour interroger l'API Roblox.");
    case "chat_state":
      return { connected: false };
    case "chat_connect":
      demoChat();
      return null;
    case "chat_disconnect":
    case "chat_send":
    case "chat_dm":
    case "chat_join":
    case "chat_leave":
    case "chat_typing":
      return null;
    default:
      throw new Error("Commande inconnue : " + cmd);
  }
}

function demoChat() {
  const push = (delay, payload) => setTimeout(() => onChatEvent(payload), delay);
  push(300, { type: "status", state: "connected", url: "ws://démo" });
  push(500, { type: "welcome", user_id: 1, color: "#ff4d45", rooms: ["general", "fastflags", "jeux", "aide", "off-topic"], joined: ["general"], history: {} });
  push(700, {
    type: "presence",
    users: [
      { id: 1, username: "Demo", color: "#ff4d45", rooms: ["general"] },
      { id: 2, username: "BloxFan42", color: "#00a2ff", rooms: ["general", "jeux"] },
      { id: 3, username: "NoobSlayer", color: "#0da84c", rooms: ["general"] },
    ],
  });
  push(900, { type: "message", room: "general", id: 10, from: "BloxFan42", from_id: 2, color: "#00a2ff", content: "Salut ! Quelqu'un pour adopt me sur Blox Fruits ? 🍇", ts: Math.floor(Date.now() / 1000) - 120 });
  push(1100, { type: "message", room: "general", id: 11, from: "NoobSlayer", from_id: 3, color: "#0da84c", content: "Moi ! Et n'oublie pas d'appliquer le preset FPS Max 😎", ts: Math.floor(Date.now() / 1000) - 60 });
}

async function invoke(cmd, args) {
  if (HAS_TAURI) return window.__TAURI__.core.invoke(cmd, args || {});
  return mockInvoke(cmd, args);
}

async function listenChat(handler) {
  if (HAS_TAURI) {
    await window.__TAURI__.event.listen("chat-event", (e) => handler(e.payload));
  }
}

/* ───────────────────────── État global ───────────────────────── */

const state = {
  settings: null,
  roblox: null, // statut lanceur
  flagsFile: null,
  currentFlags: {}, // flags réellement sur le disque
  pendingFlags: {}, // flags en cours d'édition
  catalog: [],
  presets: [],
  ffCategory: "all",
  ffSearch: "",
  ffTab: "catalog",
  profileLoading: false,
  chat: {
    connected: false,
    status: "offline",
    rooms: [],
    joined: new Set(),
    activeRoom: "general",
    activeThread: null, // null = salon, sinon pseudo DM
    users: [],
    history: {}, // room -> [msgs]
    threads: {}, // username -> [msgs]
    unread: {}, // room | "dm:X" -> n
    typing: {}, // room -> {name: expiryTs}
    myName: null,
    myColor: "#4b5563",
    lastTypingSent: 0,
  },
};

/* ───────────────────────── Toasts & modale ───────────────────────── */

function toast(msg, kind) {
  const box = el("div", "toast" + (kind ? " " + kind : ""), msg);
  $("#toasts").appendChild(box);
  setTimeout(() => box.remove(), kind === "err" ? 6000 : 4000);
}

function confirmModal(textKey, vars, okLabelKey) {
  return new Promise((resolve) => {
    const backdrop = $("#modal-backdrop");
    $("#modal-text").textContent = t(textKey, vars);
    const ok = $("#modal-ok");
    const cancel = $("#modal-cancel");
    ok.textContent = t(okLabelKey || "common.confirm");
    backdrop.classList.remove("hidden");
    const done = (val) => {
      backdrop.classList.add("hidden");
      ok.onclick = null;
      cancel.onclick = null;
      resolve(val);
    };
    ok.onclick = () => done(true);
    cancel.onclick = () => done(false);
  });
}

/* ───────────────────────── Routeur ───────────────────────── */

function setView(name) {
  $("#app").dataset.view = name;
  $$(".view").forEach((v) => v.classList.add("hidden"));
  $("#view-" + name).classList.remove("hidden");
  $$(".rail-btn").forEach((b) => b.classList.toggle("active", b.dataset.view === name));
  if (name === "home") refreshHome();
  if (name === "chat") {
    renderChatAll();
    scrollMessages(true);
  }
}

/* ───────────────────────── Accueil ───────────────────────── */

async function refreshHome() {
  try {
    state.roblox = await invoke("roblox_status");
  } catch {
    return;
  }
  renderHomeStatus();
  renderHomeChat();
}

function renderHomeStatus() {
  const box = $("#roblox-status-body");
  box.textContent = "";
  const r = state.roblox || { installed: false, running: false };

  const pill = el("span", "pill " + (r.running ? "ok" : r.installed ? "warn" : "ko"));
  pill.appendChild(el("span", "dot"));
  pill.appendChild(el("span", null, r.running ? t("home.running") : r.installed ? t("home.installed_yes") : t("home.installed_no")));
  box.appendChild(pill);

  if (r.installed) {
    const rowV = el("div", "status-row");
    rowV.appendChild(el("span", "k", t("home.version")));
    rowV.appendChild(el("span", "v", r.version_dir || "—"));
    box.appendChild(rowV);

    const rowP = el("div", "status-row");
    rowP.appendChild(el("span", "k", t("home.path")));
    const pv = el("span", "v wrap subtle", truncateMiddle(r.player_path || "—", 64));
    pv.title = r.player_path || "";
    rowP.appendChild(pv);
    box.appendChild(rowP);
  } else {
    box.appendChild(el("p", "subtle", t("home.no_roblox_hint")));
  }

  const rowF = el("div", "status-row");
  rowF.appendChild(el("span", "k", "FastFlags"));
  rowF.appendChild(el("span", "v", Object.keys(state.currentFlags).length + " " + t("home.flags_active")));
  box.appendChild(rowF);
}

function renderHomeChat() {
  const box = $("#home-chat-body");
  box.textContent = "";
  const c = state.chat;
  const pill = el("span", "pill " + (c.connected ? "ok" : "ko"));
  pill.appendChild(el("span", "dot"));
  pill.appendChild(el("span", null, c.connected ? t("home.chat_online") : t("home.chat_offline", { url: truncateMiddle(state.settings?.chat_server_url || "", 34) })));
  box.appendChild(pill);
}

function bindHome() {
  $("#btn-launch").addEventListener("click", async () => {
    const btn = $("#btn-launch");
    btn.disabled = true;
    try {
      await invoke("launch_roblox", { placeId: null });
      toast(t("home.launched_ok"), "ok");
    } catch (e) {
      toast(t("home.launch_error", { e: String(e) }), "err");
    }
    btn.disabled = false;
    setTimeout(refreshHome, 1500);
  });

  $("#join-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const id = parseInt($("#place-id").value, 10);
    if (!id || id <= 0) {
      toast(t("home.bad_place"), "err");
      return;
    }
    try {
      await invoke("launch_roblox", { placeId: id });
      toast(t("home.joining", { id }), "ok");
    } catch (err) {
      toast(t("home.launch_error", { e: String(err) }), "err");
    }
  });

  $("#btn-manage-flags").addEventListener("click", () => setView("fflags"));
  $("#btn-go-chat").addEventListener("click", () => setView("chat"));
}

/* ───────────────────────── FastFlags ───────────────────────── */

const CATS = ["performances", "graphismes", "moteur", "reseau", "telemetrie", "divers"];

function sortedify(obj) {
  const keys = Object.keys(obj).sort();
  return JSON.stringify(keys.map((k) => [k, obj[k]]));
}

function isDirty() {
  return sortedify(state.currentFlags) !== sortedify(state.pendingFlags);
}

function parseValueText(raw) {
  const s = String(raw).trim();
  if (s === "true") return true;
  if (s === "false") return false;
  if (s !== "" && !isNaN(Number(s))) return Number(s);
  return s;
}

function valueToText(v) {
  if (typeof v === "boolean") return v ? "true" : "false";
  return String(v);
}

function renderFfHeader() {
  $("#ff-count-badge").textContent = Object.keys(state.pendingFlags).length;
  $("#ff-dirty-badge").classList.toggle("hidden", !isDirty());
  $("#ff-file-line").textContent = state.flagsFile ? "📄 " + truncateMiddle(state.flagsFile, 70) : t("ff.no_roblox");
}

function renderCatalog() {
  const list = $("#ff-catalog-list");
  list.textContent = "";
  const q = state.ffSearch.trim().toLowerCase();
  const entries = state.catalog.filter((e) => {
    const okCat = state.ffCategory === "all" || e.category === state.ffCategory;
    const okQ = !q || e.key.toLowerCase().includes(q) || e.desc_fr.toLowerCase().includes(q) || e.desc_en.toLowerCase().includes(q);
    return okCat && okQ;
  });
  for (const entry of entries) {
    const card = el("div", "flag-card" + (state.pendingFlags[entry.key] !== undefined ? " on" : ""));
    const info = el("div", "flag-info");
    info.appendChild(el("div", "flag-key", entry.key));
    info.appendChild(el("div", "flag-desc", window.BloxHubLang === "en" ? entry.desc_en : entry.desc_fr));
    const meta = el("div", "flag-meta");
    meta.appendChild(el("span", "flag-tag", t("cat." + entry.category)));
    meta.appendChild(el("span", "flag-tag", entry.vtype));
    if (state.pendingFlags[entry.key] !== undefined && sortedify({ [entry.key]: state.pendingFlags[entry.key] }) !== sortedify({ [entry.key]: entry.value })) {
      meta.appendChild(el("span", "flag-tag", t("ff.custom")));
    }
    info.appendChild(meta);
    card.appendChild(info);

    const sw = el("label", "switch");
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = state.pendingFlags[entry.key] !== undefined;
    input.addEventListener("change", () => {
      if (input.checked) state.pendingFlags[entry.key] = JSON.parse(JSON.stringify(entry.value));
      else delete state.pendingFlags[entry.key];
      renderFfAll();
    });
    const track = el("span", "track");
    sw.appendChild(input);
    sw.appendChild(track);
    card.appendChild(sw);
    list.appendChild(card);
  }
  if (!entries.length) {
    list.appendChild(el("p", "subtle", "—"));
  }
}

function renderCats() {
  const box = $("#ff-cats");
  box.textContent = "";
  const mk = (id, label) => {
    const chip = el("button", "chip" + (state.ffCategory === id ? " active" : ""), label);
    chip.addEventListener("click", () => {
      state.ffCategory = id;
      renderCats();
      renderCatalog();
    });
    box.appendChild(chip);
  };
  mk("all", t("ff.cat_all"));
  for (const c of CATS) mk(c, t("cat." + c));
}

function renderPresets() {
  const grid = $("#preset-grid");
  grid.textContent = "";
  for (const p of state.presets) {
    const card = el("div", "preset-card");
    card.appendChild(el("h4", null, window.BloxHubLang === "en" ? p.name_en : p.name_fr));
    card.appendChild(el("div", "desc", window.BloxHubLang === "en" ? p.desc_en : p.desc_fr));
    const flags = el("div", "preset-flags");
    const keys = Object.keys(p.flags);
    flags.appendChild(el("span", "flag-tag", t("ff.flags_n", { n: keys.length })));
    for (const k of keys.slice(0, 3)) flags.appendChild(el("span", "flag-tag", k));
    if (keys.length > 3) flags.appendChild(el("span", "flag-tag", "+" + (keys.length - 3)));
    card.appendChild(flags);
    const btn = el("button", "btn", t("ff.preset_use"));
    btn.addEventListener("click", () => {
      state.pendingFlags = JSON.parse(JSON.stringify(p.flags));
      renderFfAll();
      toast(t("ff.preset_applied", { name: window.BloxHubLang === "en" ? p.name_en : p.name_fr }), "ok");
    });
    card.appendChild(btn);
    grid.appendChild(card);
  }
}

function renderAdvanced() {
  const tbody = $("#ff-adv-table tbody");
  tbody.textContent = "";
  for (const key of Object.keys(state.pendingFlags)) {
    const tr = document.createElement("tr");
    tr.dataset.key = key;

    const tdKey = document.createElement("td");
    const inKey = document.createElement("input");
    inKey.value = key;
    inKey.spellcheck = false;
    inKey.addEventListener("change", () => {
      const newKey = inKey.value.trim();
      if (!newKey || newKey === key) {
        inKey.value = key;
        return;
      }
      const oldVal = state.pendingFlags[key];
      delete state.pendingFlags[key];
      state.pendingFlags[newKey] = oldVal;
      renderFfAll();
    });
    tdKey.appendChild(inKey);
    tr.appendChild(tdKey);

    const tdVal = document.createElement("td");
    const inVal = document.createElement("input");
    inVal.value = valueToText(state.pendingFlags[key]);
    inVal.spellcheck = false;
    inVal.addEventListener("change", () => {
      state.pendingFlags[key] = parseValueText(inVal.value);
      inVal.value = valueToText(state.pendingFlags[key]);
      renderFfHeader();
      renderCatalog();
    });
    tdVal.appendChild(inVal);
    tr.appendChild(tdVal);

    const tdDel = document.createElement("td");
    const del = el("button", "row-del", "✕");
    del.title = "✕";
    del.addEventListener("click", () => {
      delete state.pendingFlags[key];
      renderFfAll();
    });
    tdDel.appendChild(del);
    tr.appendChild(tdDel);

    tbody.appendChild(tr);
  }
}

function renderFfHeaderOnly() {
  renderFfHeader();
  $("#ff-count-badge").textContent = Object.keys(state.pendingFlags).length;
  $("#ff-dirty-badge").classList.toggle("hidden", !isDirty());
}

function renderFfAll() {
  renderFfHeader();
  renderCats();
  renderCatalog();
  renderPresets();
  renderAdvanced();
  renderHomeStatus();
}

function bindFflags() {
  $$(".ff-tabs .tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      state.ffTab = tab.dataset.tab;
      $$(".ff-tabs .tab").forEach((x) => x.classList.toggle("active", x === tab));
      for (const name of ["catalog", "presets", "advanced"]) {
        $("#ff-panel-" + name).classList.toggle("hidden", name !== state.ffTab);
      }
    });
  });

  $("#ff-search").addEventListener("input", (e) => {
    state.ffSearch = e.target.value;
    renderCatalog();
  });

  $("#ff-apply").addEventListener("click", async () => {
    if (!state.flagsFile) {
      toast(t("ff.no_roblox"), "err");
      return;
    }
    const n = Object.keys(state.pendingFlags).length;
    const okGo = await confirmModal("ff.apply_confirm", { n });
    if (!okGo) return;
    try {
      await invoke("apply_flags", { flags: state.pendingFlags });
      state.currentFlags = JSON.parse(JSON.stringify(state.pendingFlags));
      renderFfAll();
      toast(t("ff.applied_ok", { n }), "ok");
    } catch (e) {
      toast(String(e), "err");
    }
  });

  $("#ff-reset").addEventListener("click", async () => {
    const okGo = await confirmModal("ff.reset_confirm", {});
    if (!okGo) return;
    try {
      await invoke("clear_flags");
      state.currentFlags = {};
      state.pendingFlags = {};
      renderFfAll();
      toast(t("ff.cleared_ok"), "ok");
    } catch (e) {
      toast(String(e), "err");
    }
  });

  $("#ff-export").addEventListener("click", async () => {
    try {
      const res = await invoke("export_flags");
      toast(t("ff.exported_ok", { path: res.path }), "ok");
    } catch (e) {
      toast(String(e), "err");
    }
  });

  $("#ff-import").addEventListener("click", () => $("#ff-import-file").click());

  $("#ff-import-file").addEventListener("change", async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = "";
    if (!file) return;
    let payload;
    try {
      payload = JSON.parse(await file.text());
    } catch {
      toast(t("ff.import_error"), "err");
      return;
    }
    if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
      toast(t("ff.import_error"), "err");
      return;
    }
    const n = Object.keys(payload).length;
    const merge = await confirmModal("ff.import_confirm", { n });
    try {
      const res = await invoke("import_flags", { payload: JSON.stringify(payload), mode: merge ? "merge" : "replace" });
      toast(t("ff.imported_ok", { total: res.total }), "ok");
      const flags = await invoke("get_flags");
      state.flagsFile = flags.file;
      state.currentFlags = flags.flags || {};
      state.pendingFlags = JSON.parse(JSON.stringify(state.currentFlags));
      renderFfAll();
    } catch (err) {
      toast(String(err), "err");
    }
  });

  $("#ff-add-row").addEventListener("click", () => {
    let i = 1;
    let key = "MonFlag" + i;
    while (state.pendingFlags[key] !== undefined) {
      i += 1;
      key = "MonFlag" + i;
    }
    state.pendingFlags[key] = true;
    renderFfAll();
  });
}

/* ───────────────────────── Profil ───────────────────────── */

function renderProfileLoading(name) {
  const box = $("#profile-result");
  box.textContent = "";
  box.appendChild(el("div", "spinner"));
  box.appendChild(el("p", "subtle", t("prof.loading", { name })));
  box.style.textAlign = "center";
}

function renderProfileError(msg) {
  const box = $("#profile-result");
  box.textContent = "";
  box.style.textAlign = "";
  box.appendChild(el("div", "error-box", msg));
}

function renderProfile(data) {
  const box = $("#profile-result");
  box.textContent = "";
  box.style.textAlign = "";
  const user = data.user || {};
  const details = data.details || {};
  const created = details.created ? new Date(details.created).toLocaleDateString() : null;

  const card = el("div", "profile-card");
  const left = el("div", "profile-avatar");
  if (data.avatarUrl) {
    const img = document.createElement("img");
    img.src = data.avatarUrl;
    img.alt = user.name || "";
    img.loading = "lazy";
    left.appendChild(img);
  }
  card.appendChild(left);

  const main = el("div", "profile-main");
  const h = el("h3");
  h.appendChild(el("span", null, user.displayName || user.name || "?"));
  if (user.hasVerifiedBadge) {
    const v = el("span", "badge", "✔ " + t("prof.verified"));
    v.style.background = "rgba(0,162,255,.15)";
    v.style.color = "#4cc2ff";
    h.appendChild(v);
  }
  if (details.isBanned) {
    const b = el("span", "badge warn", t("prof.banned"));
    h.appendChild(b);
  }
  main.appendChild(h);
  main.appendChild(el("div", "profile-username", "@" + (user.name || "") + " · ID " + (user.id || "")));

  const meta = el("div", "profile-meta");
  if (created) {
    const p1 = el("span", "pill ok");
    p1.textContent = t("prof.created") + " " + created;
    meta.appendChild(p1);
  }
  const p2 = el("span", "pill " + (details.isBanned ? "warn" : "ok"));
  p2.textContent = details.isBanned ? t("prof.banned") : t("prof.not_banned");
  meta.appendChild(p2);
  main.appendChild(meta);

  main.appendChild(el("div", "profile-desc", details.description || t("prof.no_description")));
  card.appendChild(main);
  box.appendChild(card);

  const friends = data.friends || [];
  box.appendChild(el("div", "friends-title", t("prof.friends", { n: friends.length })));
  if (!friends.length) {
    box.appendChild(el("p", "subtle", t("prof.no_friends")));
    return;
  }
  const grid = el("div", "friends-grid");
  for (const f of friends) {
    const fc = el("div", "friend-card");
    if (f.imageUrl) {
      const img = document.createElement("img");
      img.src = f.imageUrl;
      img.alt = f.name || "";
      img.loading = "lazy";
      fc.appendChild(img);
    }
    fc.appendChild(el("div", "fname", f.displayName || f.name || ""));
    fc.title = "@" + (f.name || "");
    fc.addEventListener("click", () => {
      $("#profile-username").value = f.name || "";
      $("#profile-form").dispatchEvent(new Event("submit"));
    });
    grid.appendChild(fc);
  }
  box.appendChild(grid);
}

function bindProfile() {
  $("#profile-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    if (state.profileLoading) return;
    const name = $("#profile-username").value.trim();
    if (!name) return;
    state.profileLoading = true;
    renderProfileLoading(name);
    try {
      const data = await invoke("roblox_profile", { username: name });
      renderProfile(data);
    } catch (err) {
      const msg = String(err);
      renderProfileError(/introuvable|not found/i.test(msg) ? t("prof.notfound") : msg);
    }
    state.profileLoading = false;
  });
}

/* ───────────────────────── Paramètres ───────────────────────── */

function renderSettings() {
  const s = state.settings;
  if (!s) return;
  $("#set-chat-url").value = s.chat_server_url;
  $("#set-chat-username").value = s.chat_username;
  $("#set-autoconnect").checked = !!s.auto_connect_chat;
  $("#set-custom-path").value = s.roblox_custom_path || "";
  $$('input[name="lang"]').forEach((r) => {
    r.checked = r.value === window.BloxHubLang;
    r.closest(".lang-opt").classList.toggle("checked", r.checked);
  });
}

function setLang(lang, rerender) {
  window.BloxHubLang = lang === "en" ? "en" : "fr";
  localStorage.setItem("bloxhub.lang", window.BloxHubLang);
  window.BloxHubApplyI18n();
  if (rerender) {
    renderCats();
    renderCatalog();
    renderPresets();
    renderHomeStatus();
    renderHomeChat();
    renderChatAll();
  }
}

function bindSettings() {
  $$('input[name="lang"]').forEach((r) => {
    r.addEventListener("change", () => {
      setLang(r.value, true);
      renderSettings();
    });
  });

  $("#set-save").addEventListener("click", async () => {
    const custom = $("#set-custom-path").value.trim();
    const s = {
      language: window.BloxHubLang,
      chat_server_url: $("#set-chat-url").value.trim() || "ws://127.0.0.1:8787",
      chat_username: $("#set-chat-username").value.trim() || "Joueur",
      auto_connect_chat: $("#set-autoconnect").checked,
      roblox_custom_path: custom ? custom : null,
    };
    try {
      await invoke("save_settings", { settings: s });
      state.settings = s;
      toast(t("set.saved"), "ok");
      renderHomeChat();
    } catch (e) {
      toast(String(e), "err");
    }
  });
}

/* ───────────────────────── Chat ───────────────────────── */

function connBadgeClass() {
  const s = state.chat.status;
  if (state.chat.connected) return "conn-badge on";
  if (s === "connecting" || s === "reconnecting") return "conn-badge wait";
  return "conn-badge off";
}

function connBadgeText() {
  if (state.chat.connected) return t("chat.connected");
  const s = state.chat.status;
  if (s === "connecting") return t("chat.connecting");
  if (s === "reconnecting") return t("chat.reconnect_soon");
  if (s === "error") return t("chat.error");
  return t("chat.offline");
}

function renderConnBadges() {
  for (const id of ["chat-conn-badge", "chat-conn-badge2"]) {
    const b = document.getElementById(id);
    b.className = connBadgeClass();
    b.textContent = connBadgeText();
  }
  renderHomeChat();
}

function roomDisplayName(room) {
  const key = "room." + room;
  const label = t(key);
  return label === key ? room : label;
}

function totalUnread() {
  return Object.values(state.chat.unread).reduce((a, b) => a + b, 0);
}

function renderSidepanel() {
  const chanBox = $("#channel-list");
  chanBox.textContent = "";
  const rooms = state.chat.rooms.length ? state.chat.rooms : ["general"];
  for (const room of rooms) {
    const joined = state.chat.joined.has(room);
    const item = el("button", "channel-item" + (state.chat.activeThread === null && state.chat.activeRoom === room ? " active" : ""));
    item.appendChild(el("span", "hash", "#"));
    item.appendChild(el("span", null, roomDisplayName(room)));
    if (!joined) {
      const tag = el("span", "flag-tag", "—");
      item.appendChild(tag);
    }
    const unread = state.chat.unread[room] || 0;
    if (unread > 0) item.appendChild(el("span", "unread", String(unread)));
    item.addEventListener("click", () => switchRoom(room));
    chanBox.appendChild(item);
  }

  const dmBox = $("#dm-list");
  dmBox.textContent = "";
  const dmNames = Object.keys(state.chat.threads);
  if (!dmNames.length) {
    dmBox.appendChild(el("p", "subtle", t("chat.no_dms")));
  }
  for (const name of dmNames) {
    const msgs = state.chat.threads[name];
    const last = msgs[msgs.length - 1];
    const color = (state.chat.users.find((u) => u.username === name) || {}).color || "#4b5563";
    const item = el("button", "channel-item" + (state.chat.activeThread === name ? " active" : ""));
    item.appendChild(avatarEl(name, color, "dm-avatar"));
    item.appendChild(el("span", null, name));
    const unread = state.chat.unread["dm:" + name] || 0;
    if (unread > 0) item.appendChild(el("span", "unread", String(unread)));
    item.title = last ? last.content : "";
    item.addEventListener("click", () => openThread(name));
    dmBox.appendChild(item);
  }

  $("#rail-chat-dot").classList.toggle("hidden", totalUnread() === 0 || $("#app").dataset.view === "chat");
}

function renderMembers() {
  const list = $("#members-list");
  list.textContent = "";
  const c = state.chat;
  const users = c.activeThread === null ? c.users.filter((u) => (u.rooms || []).includes(c.activeRoom)) : c.users.slice();
  const sorted = users.slice().sort((a, b) => a.username.localeCompare(b.username, "fr"));
  for (const u of sorted) {
    const item = el("div", "member-item");
    item.appendChild(avatarEl(u.username, u.color, "dm-avatar"));
    item.appendChild(el("span", "name", u.username + (u.id === c.myId ? " " + t("chat.you") : "")));
    item.appendChild(el("span", "dm-hint", "DM"));
    item.title = t("chat.open_dm_hint") || "";
    item.addEventListener("click", () => openThread(u.username));
    list.appendChild(item);
  }
  $("#members-count").textContent = t("chat.online_n", { n: sorted.length });
}

function activeMessages() {
  const c = state.chat;
  if (c.activeThread) return c.threads[c.activeThread] || [];
  return c.history[c.activeRoom] || [];
}

function renderMessages() {
  const box = $("#messages");
  box.textContent = "";
  const msgs = activeMessages();
  const c = state.chat;

  if (!msgs.length) {
    const ph = el("div", "chat-placeholder");
    ph.appendChild(el("div", "big", c.activeThread ? "✉️" : "#"));
    ph.appendChild(
      el(
        "p",
        null,
        c.activeThread ? t("chat.empty_thread", { name: c.activeThread }) : t("chat.empty_room")
      )
    );
    box.appendChild(ph);
  }

  let lastAuthor = null;
  let lastTs = 0;
  let lastDay = "";
  for (const m of msgs) {
    if (m.system) {
      lastAuthor = null;
      const sys = el("div", "msg-system", (m.event === "join" ? t("chat.user_joined", { name: m.username }) : t("chat.user_left", { name: m.username })));
      box.appendChild(sys);
      continue;
    }
    const day = fmtDay(m.ts);
    if (day && day !== lastDay) {
      box.appendChild(el("div", "chat-day-sep", day));
      lastDay = day;
      lastAuthor = null;
    }
    const compact = lastAuthor === m.from && m.ts - lastTs < 300 && lastDay === day;
    const group = el("div", "msg-group" + (compact ? " compact" : " first") + (m.from_id === c.myId ? " self" : ""));
    if (!compact) {
      group.appendChild(avatarEl(m.from, m.color));
      const body = el("div", "msg-body");
      const head = el("div", "msg-head");
      head.appendChild(el("span", "msg-author", m.from + (m.from_id === c.myId ? "" : "")));
      head.appendChild(el("span", "msg-time", fmtTime(m.ts)));
      body.appendChild(head);
      body.appendChild(el("div", "msg-content", m.content));
      group.appendChild(body);
    } else {
      const body = el("div", "msg-body");
      body.appendChild(el("div", "msg-content", m.content));
      body.title = fmtTime(m.ts);
      group.appendChild(body);
    }
    lastAuthor = m.from;
    lastTs = m.ts;
    box.appendChild(group);
  }
}

function renderTypingBar() {
  const c = state.chat;
  const bar = $("#typing-bar");
  const now = Date.now();
  const names = Object.entries(c.typing[c.activeRoom] || {})
    .filter(([, exp]) => exp > now)
    .map(([name]) => name);
  if (!names.length || c.activeThread) {
    bar.classList.add("hidden");
    return;
  }
  bar.classList.remove("hidden");
  bar.textContent =
    names.length === 1
      ? t("chat.typing", { name: names[0] })
      : t("chat.typing_many", { name: names[0], n: names.length - 1 });
}

function renderChatRoomTitle() {
  const c = state.chat;
  const title = $("#chat-room-title");
  if (c.activeThread) {
    title.textContent = "@" + c.activeThread;
  } else {
    title.textContent = "#" + roomDisplayName(c.activeRoom);
  }
}

function renderChatAll() {
  renderSidepanel();
  renderMembers();
  renderMessages();
  renderTypingBar();
  renderChatRoomTitle();
  renderConnBadges();
}

function scrollMessages(force) {
  const box = $("#messages");
  const nearBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 140;
  if (force || nearBottom) box.scrollTop = box.scrollHeight;
}

function markRead(key) {
  delete state.chat.unread[key];
  renderSidepanel();
}

function switchRoom(room) {
  const c = state.chat;
  c.activeRoom = room;
  c.activeThread = null;
  if (!c.joined.has(room)) {
    c.joined.add(room);
    invoke("chat_join", { room }).catch(() => {});
  }
  markRead(room);
  renderChatAll();
  scrollMessages(true);
}

function openThread(name) {
  const c = state.chat;
  c.activeThread = name;
  if (!c.threads[name]) c.threads[name] = [];
  markRead("dm:" + name);
  renderChatAll();
  scrollMessages(true);
}

async function chatConnect() {
  if (state.chat.status === "connecting" || state.chat.connected) return;
  state.chat.status = "connecting";
  renderConnBadges();
  try {
    await invoke("chat_connect");
  } catch (e) {
    state.chat.status = "offline";
    renderConnBadges();
    toast(String(e), "err");
  }
}

function onChatEvent(ev) {
  const c = state.chat;
  const view = $("#app").dataset.view;
  switch (ev.type) {
    case "status": {
      const s = ev.state;
      if (s === "connected") {
        c.connected = true;
        c.status = "connected";
      } else if (s === "connecting") {
        c.connected = false;
        c.status = "connecting";
      } else if (s === "error") {
        c.connected = false;
        c.status = s === "error" && c.connected === false ? "error" : "error";
        if (view !== "chat") toast(t("home.chat_offline", { url: ev.url || "" }), "err");
      } else if (s === "disconnected") {
        c.connected = false;
        c.status = c.status === "connected" ? "reconnecting" : c.status === "offline" ? "offline" : "reconnecting";
      } else if (s === "offline") {
        c.connected = false;
        c.status = "offline";
      }
      renderConnBadges();
      break;
    }
    case "welcome": {
      c.myId = ev.user_id;
      c.myColor = ev.color;
      c.rooms = ev.rooms || [];
      c.joined = new Set(ev.joined || ["general"]);
      if (!c.rooms.includes(c.activeRoom)) c.activeRoom = "general";
      const hist = ev.history || {};
      for (const [room, msgs] of Object.entries(hist)) {
        c.history[room] = msgs;
      }
      renderChatAll();
      break;
    }
    case "presence": {
      c.users = ev.users || [];
      renderMembers();
      renderSidepanel();
      break;
    }
    case "message": {
      const room = ev.room;
      if (!c.history[room]) c.history[room] = [];
      c.history[room].push(ev);
      delete c.typing[room]?.[ev.from];
      const isActive = view === "chat" && c.activeThread === null && c.activeRoom === room;
      if (!isActive) c.unread[room] = (c.unread[room] || 0) + 1;
      if (view === "chat") {
        renderSidepanel();
        renderTypingBar();
        if (isActive) {
          renderMessages();
          scrollMessages(false);
        } else {
          renderChatRoomTitle();
        }
      } else {
        renderSidepanel();
      }
      break;
    }
    case "dm": {
      const other = ev.from_self ? ev.to : ev.from;
      if (!c.threads[other]) c.threads[other] = [];
      c.threads[other].push(ev);
      const isActive = view === "chat" && c.activeThread === other;
      if (!isActive) c.unread["dm:" + other] = (c.unread["dm:" + other] || 0) + 1;
      if (view === "chat") {
        renderSidepanel();
        if (isActive) {
          renderMessages();
          scrollMessages(false);
        }
      } else {
        renderSidepanel();
      }
      break;
    }
    case "system": {
      const room = ev.room || "general";
      if (!c.history[room]) c.history[room] = [];
      c.history[room].push({ system: true, event: ev.event, username: ev.username, ts: Math.floor(Date.now() / 1000) });
      if (view === "chat" && c.activeThread === null && c.activeRoom === room) {
        renderMessages();
        scrollMessages(false);
      }
      break;
    }
    case "typing": {
      if (!c.typing[ev.room]) c.typing[ev.room] = {};
      c.typing[ev.room][ev.from] = Date.now() + 4000;
      setTimeout(renderTypingBar, 4200);
      renderTypingBar();
      break;
    }
    case "error": {
      toast(t("chat.error_server", { detail: ev.detail || ev.code || "?" }), "err");
      break;
    }
    default:
      break;
  }
}

function bindChat() {
  $("#chat-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const input = $("#chat-input");
    const content = input.value.trim();
    if (!content) return;
    const c = state.chat;
    try {
      if (c.activeThread) await invoke("chat_dm", { to: c.activeThread, content });
      else await invoke("chat_send", { room: c.activeRoom, content });
      input.value = "";
    } catch {
      toast(t("chat.send_failed"), "err");
    }
  });

  let typingTimer = null;
  $("#chat-input").addEventListener("input", () => {
    const c = state.chat;
    if (c.activeThread) return;
    const now = Date.now();
    if (now - c.lastTypingSent > 2000 && c.connected) {
      c.lastTypingSent = now;
      invoke("chat_typing", { room: c.activeRoom }).catch(() => {});
    }
    clearTimeout(typingTimer);
  });
}

/* ───────────────────────── Initialisation ───────────────────────── */

function bindRail() {
  $$(".rail-btn").forEach((btn) => {
    btn.addEventListener("click", () => setView(btn.dataset.view));
  });
}

async function init() {
  const stored = localStorage.getItem("bloxhub.lang");
  window.BloxHubLang = stored || ((navigator.language || "fr").startsWith("en") ? "en" : "fr");

  bindRail();
  bindHome();
  bindFflags();
  bindProfile();
  bindSettings();
  bindChat();

  try {
    const s = await invoke("get_settings");
    state.settings = s;
    if (!stored && (s.language === "fr" || s.language === "en")) {
      window.BloxHubLang = s.language;
      localStorage.setItem("bloxhub.lang", window.BloxHubLang);
    }
  } catch (e) {
    toast(String(e), "err");
  }

  window.BloxHubApplyI18n();

  try {
    const [catalog, presets, flags, status] = await Promise.all([
      invoke("flag_catalog"),
      invoke("flag_presets"),
      invoke("get_flags").catch(() => ({ file: null, flags: {} })),
      invoke("roblox_status").catch(() => null),
    ]);
    state.catalog = catalog || [];
    state.presets = presets || [];
    state.flagsFile = flags.file || null;
    state.currentFlags = flags.flags || {};
    state.pendingFlags = JSON.parse(JSON.stringify(state.currentFlags));
    state.roblox = status;
  } catch (e) {
    toast(String(e), "err");
  }

  renderSettings();
  renderFfAll();
  renderHomeStatus();
  renderHomeChat();
  renderChatAll();

  await listenChat(onChatEvent);

  if (state.settings && state.settings.auto_connect_chat) {
    chatConnect();
  }

  setInterval(() => {
    if ($("#app").dataset.view === "home") refreshHome();
  }, 10000);
}

document.addEventListener("DOMContentLoaded", init);
