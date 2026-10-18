"use strict";

const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;
const state = {
  page: "overview",
  accounts: [],
  instances: [],
  settings: { language: "en", ramMb: 4096, javaPath: null, serverAddress: "" },
  versions: [],
  selectedInstance: null,
  logs: [],
  activeInstall: null,
  modResults: [],
  installedMods: [],
  crashReports: [],
};

const icons = {
  overview: '<svg viewBox="0 0 24 24"><path d="M4 13h7V4H4zM15 20h5v-7h-5zM15 9h5V4h-5zM4 20h7v-3H4z"/></svg>',
  instances: '<svg viewBox="0 0 24 24"><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z"/><path d="m4.5 7.7 7.5 4.2 7.5-4.2M12 12v8.5"/></svg>',
  mods: '<svg viewBox="0 0 24 24"><path d="M8 3v4M16 3v4M4 9h16v11H4z"/><path d="M8 13h.01M12 13h.01M16 13h.01M8 17h.01M12 17h.01M16 17h.01"/></svg>',
  activity: '<svg viewBox="0 0 24 24"><path d="M3 12h4l3-8 4 16 3-8h4"/></svg>',
  settings: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="m19.4 15 .1.1 1.1.9-1.2 2.1-1.4-.5-.2.1-1.3.8-.3 1.5h-2.4l-.3-1.5-1.3-.8-.2-.1-1.4.5-1.2-2.1 1.1-.9.1-.2v-1.5l-.1-.2-1.1-.9 1.2-2.1 1.4.5.2-.1 1.3-.8.3-1.5h2.4l.3 1.5 1.3.8.2.1 1.4-.5 1.2 2.1-1.1.9-.1.2z"/></svg>',
  cube: '<svg viewBox="0 0 24 24"><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z"/><path d="m4.5 7.7 7.5 4.2 7.5-4.2M12 12v9"/></svg>',
  play: '<svg viewBox="0 0 24 24"><path d="m8 5 11 7-11 7z"/></svg>',
  plus: '<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>',
};

const arabicText = {
  "Overview": "نظرة عامة",
  "Instances": "النسخ",
  "Mod browser": "متصفح الإضافات",
  "Game activity": "نشاط اللعبة",
  "Settings": "الإعدادات",
  "Workspace": "مساحة العمل",
  "Java launcher": "مشغّل جافا",
  "No account": "لا يوجد حساب",
  "Microsoft account": "حساب مايكروسوفت",
  "Offline account": "حساب غير متصل",
  "Sign in to play": "سجّل الدخول للعب",
  "Manage accounts": "إدارة الحسابات",
  "LOCAL LIBRARY": "المكتبة المحلية",
  "Add account": "إضافة حساب",
  "THE WORLD IS YOURS": "العالم لك",
  "A little light for your next big adventure.": "نورٌ صغير لمغامرتك الكبيرة القادمة.",
  "Pick up where you left off, or build a new world from the ground up.": "تابع من حيث توقفت أو أنشئ عالماً جديداً من البداية.",
  "New instance": "نسخة جديدة",
  "Browse versions": "تصفّح الإصدارات",
  "INSTANCES": "النسخ",
  "ACCOUNTS": "الحسابات",
  "PROFILE": "الملف الشخصي",
  "READY": "جاهز",
  "SETUP": "الإعداد",
  "Your instances": "نسخك",
  "Separate game folders for versions, loaders, mods, and saves.": "مجلدات مستقلة للإصدارات والمحمّلات والإضافات والعوالم المحفوظة.",
  "Import .mrpack": "استيراد ‎.mrpack",
  "Create": "إنشاء",
  "Your library is ready for a first world": "مكتبتك جاهزة لعالمك الأول",
  "Create an instance and Lumen will download the selected Minecraft version into its library.": "أنشئ نسخة وسيحمّل Lumen إصدار Minecraft المحدد إلى مكتبتك.",
  "Create an instance": "إنشاء نسخة",
  "Play": "العب",
  "LAST PLAYED": "آخر لعب",
  "YOUR LIBRARY": "مكتبتك",
  "Import ZIP": "استيراد ZIP",
  "Every instance keeps its own version, mods, worlds, resource packs, memory allocation, and launch settings.": "لكل نسخة إصدارها وإضافاتها وعوالمها وحزم مواردها وذاكرتها وإعدادات تشغيلها.",
  "No instances yet": "لا توجد نسخ بعد",
  "Create a clean install or import a saved instance to get started.": "أنشئ تثبيتاً جديداً أو استورد نسخة محفوظة للبدء.",
  "Search compatible mods for the selected Minecraft version and loader. Required dependencies are installed automatically.": "ابحث عن إضافات متوافقة مع إصدار Minecraft والمحمّل المحددين. تُثبّت المتطلبات تلقائياً.",
  "Select an instance first": "اختر نسخة أولاً",
  "Create an instance before searching for compatible mods.": "أنشئ نسخة قبل البحث عن إضافات متوافقة.",
  "Go to instances": "انتقل إلى النسخ",
  "Target instance": "النسخة المستهدفة",
  "Search": "بحث",
  "Find your next favorite mod": "اعثر على إضافتك المفضلة التالية",
  "Installed files": "الملفات المثبتة",
  "Disable a mod without deleting it from the instance.": "عطّل إضافة دون حذفها من النسخة.",
  "Refresh": "تحديث",
  "No mod files found in this instance.": "لا توجد ملفات إضافات في هذه النسخة.",
  "Install": "تثبيت",
  "Disabled": "معطّل",
  "Enabled": "مفعّل",
  "Enable": "تفعيل",
  "Disable": "تعطيل",
  "DIAGNOSTICS": "التشخيص",
  "Live output from launched instances and crash reports stored in your launcher data directory.": "مخرجات النسخ المشغّلة وتقارير الأعطال المحفوظة في مجلد بيانات المشغّل.",
  "Clear console": "مسح وحدة التحكم",
  "Live console": "وحدة التحكم المباشرة",
  "Console output appears while a game process is running.": "تظهر المخرجات أثناء تشغيل اللعبة.",
  "IDLE": "خامل",
  "No game process output yet. Launch an instance to stream its log here.": "لا توجد مخرجات بعد. شغّل نسخة لعرض سجلها هنا.",
  "Crash reports": "تقارير الأعطال",
  "Reports are read from each instance's Minecraft crash-reports folder.": "تُقرأ التقارير من مجلد الأعطال الخاص بكل نسخة.",
  "Refresh reports": "تحديث التقارير",
  "Report": "التقرير",
  "Modified": "تاريخ التعديل",
  "Read": "قراءة",
  "No crash reports found.": "لم يتم العثور على تقارير أعطال.",
  "PREFERENCES": "التفضيلات",
  "Tune runtime memory, Java selection, language, and quick-join behavior.": "اضبط ذاكرة التشغيل وإصدار Java واللغة والاتصال السريع بالخادم.",
  "Game defaults": "الإعدادات الافتراضية للعبة",
  "These apply to new instances unless overridden.": "تُطبّق على النسخ الجديدة ما لم يتم تغييرها.",
  "Memory allocation": "تخصيص الذاكرة",
  "Custom Java executable": "ملف Java التنفيذي المخصص",
  "Leave blank to download a compatible Eclipse Temurin runtime automatically.": "اتركه فارغاً لتنزيل بيئة Eclipse Temurin المتوافقة تلقائياً.",
  "Quick-join server": "الخادم للاتصال السريع",
  "Optional host name or host:port. Leave blank to launch to the main menu.": "اسم مضيف أو host:port اختياري. اتركه فارغاً للانتقال إلى القائمة الرئيسية.",
  "Interface language": "لغة الواجهة",
  "Arabic uses a right-to-left layout. Game language is controlled inside Minecraft.": "تستخدم العربية تخطيطاً من اليمين إلى اليسار. تُضبط لغة اللعبة من داخل Minecraft.",
  "Save preferences": "حفظ التفضيلات",
  "Accounts": "الحسابات",
  "Microsoft credentials stay in your operating system keyring.": "تبقى بيانات اعتماد Microsoft في مخزن مفاتيح نظام التشغيل.",
  "Manage": "إدارة",
  "No accounts yet": "لا توجد حسابات بعد",
  "Add a Microsoft account or a profile for offline-mode servers.": "أضف حساب Microsoft أو ملفاً شخصياً لخوادم الوضع غير المتصل.",
  "Launcher data location": "موقع بيانات المشغّل",
  "Active": "نشط",
  "Use": "استخدام",
  "Remove": "إزالة",
  "Close": "إغلاق",
  "Cancel": "إلغاء",
  "Create instance": "إنشاء نسخة",
  "Instance name": "اسم النسخة",
  "Minecraft version": "إصدار Minecraft",
  "Install instance": "تثبيت النسخة",
  "Choose an official Minecraft version. The vanilla game files will be installed before the instance is added.": "اختر إصدار Minecraft رسمياً. ستُثبّت ملفات اللعبة الأساسية قبل إضافة النسخة.",
  "Search mods by name or feature": "ابحث عن الإضافات بالاسم أو الميزة",
  "Path to java or java.exe": "مسار java أو java.exe",
};

function translateText(value) {
  return state.settings.language === "ar" ? (arabicText[value.trim()] || value) : value;
}

function applyArabic(root) {
  if (state.settings.language !== "ar" || !root) return;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    const original = node.nodeValue || "";
    const trimmed = original.trim();
    if (arabicText[trimmed]) {
      node.nodeValue = original.replace(trimmed, arabicText[trimmed]);
    }
  }
  root.querySelectorAll("[placeholder], [title], [aria-label]").forEach((element) => {
    for (const attribute of ["placeholder", "title", "aria-label"]) {
      const value = element.getAttribute(attribute);
      if (value && arabicText[value.trim()]) element.setAttribute(attribute, arabicText[value.trim()]);
    }
  });
}

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[ch]);
}

function initials(name) {
  return (name || "LL").slice(0, 2).toUpperCase();
}

function activeAccount() {
  return state.accounts.find((account) => account.active) || null;
}

function selectedInstance() {
  return state.instances.find((instance) => instance.id === state.selectedInstance) || state.instances[0] || null;
}

function avatar(account, size = 34) {
  if (account?.skinUrl) {
    return `<div class="avatar" style="width:${size}px;height:${size}px"><img src="${esc(account.skinUrl)}" alt="" onerror="this.remove()"></div>`;
  }
  return `<div class="avatar" style="width:${size}px;height:${size}px">${esc(initials(account?.username))}</div>`;
}

function toast(message, isError = false) {
  let stack = document.querySelector(".toast-stack");
  if (!stack) {
    stack = document.createElement("div");
    stack.className = "toast-stack";
    document.body.append(stack);
  }
  const item = document.createElement("div");
  item.className = `toast${isError ? " error" : ""}`;
  item.textContent = translateText(message);
  stack.append(item);
  setTimeout(() => item.remove(), 4600);
}

function setBusy(button, busy, label) {
  if (!button) return;
  button.disabled = busy;
  if (busy) {
    button.dataset.oldText = button.textContent;
    button.textContent = label || "Working…";
  } else if (button.dataset.oldText) {
    button.textContent = button.dataset.oldText;
    delete button.dataset.oldText;
  }
}

function formatDate(seconds) {
  if (!seconds) return "NEVER LAUNCHED";
  return new Date(seconds * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" }).toUpperCase();
}

function sidebar() {
  const nav = [
    ["overview", "Overview"],
    ["instances", "Instances"],
    ["mods", "Mod browser"],
    ["activity", "Game activity"],
    ["settings", "Settings"],
  ];
  const account = activeAccount();
  return `<aside class="sidebar">
    <div class="brand"><div class="brand-mark"><span></span><span></span><span></span><span></span></div><div><div class="brand-title">Lumen</div><div class="brand-subtitle">Java launcher</div></div></div>
    <div class="nav-label">Workspace</div>
    <nav class="nav">${nav.map(([id, label]) => `<button class="nav-button ${state.page === id ? "active" : ""}" data-page="${id}"><span class="nav-icon">${icons[id]}</span>${label}</button>`).join("")}</nav>
    <div class="sidebar-bottom">
      <div class="account-mini">
        ${avatar(account, 34)}
        <div class="account-mini-copy"><div class="account-mini-name">${esc(account?.username || "No account")}</div><div class="account-mini-type">${account ? (account.accountType === "microsoft" ? "Microsoft account" : "Offline account") : "Sign in to play"}</div></div>
        <button class="add-account" id="sidebar-add-account" title="Manage accounts" aria-label="Manage accounts">+</button>
      </div>
      <div class="sidebar-foot"><span>LUMEN 0.1.0</span><span>JAVA EDITION</span></div>
    </div>
  </aside>`;
}

function header() {
  const account = activeAccount();
  return `<header class="topbar">
    <div class="crumb">LUMEN <span style="color:#4f5a51">/</span> ${esc(state.page.replace("-", " ").toUpperCase())}</div>
    <div class="top-actions">
      <span class="status-pill"><span class="status-dot"></span>LOCAL LIBRARY</span>
      <button class="account-switch" id="account-switch" aria-label="Switch account">
        ${avatar(account, 26)}<span class="account-switch-name">${esc(account?.username || "Add account")}</span><span class="account-switch-chevron">⌄</span>
      </button>
    </div>
  </header>`;
}

function instanceCard(instance, index) {
  const selected = state.selectedInstance === instance.id;
  return `<article class="instance-card ${selected ? "selected" : ""}" data-instance-card="${esc(instance.id)}" style="animation-delay:${index * 45}ms">
    <div class="instance-card-top"><div class="instance-cube">${icons.cube}</div><button class="instance-menu" data-instance-menu="${esc(instance.id)}" title="Instance options" aria-label="Instance options">···</button></div>
    <h3>${esc(instance.name)}</h3>
    <div class="instance-meta"><span>${esc(instance.gameVersionId || instance.versionId)}</span><span>·</span><span>${esc(instance.loader || "Vanilla")}</span><span class="tag ${instance.loader ? "green" : ""}">${esc(instance.loader || "Vanilla")}</span></div>
    <div class="instance-footer"><span class="last-played">LAST PLAYED ${formatDate(instance.lastPlayedAt)}</span>
      <button class="button primary small" data-launch="${esc(instance.id)}">${icons.play} Play</button>
    </div>
  </article>`;
}

function overviewPage() {
  const account = activeAccount();
  return `<div class="content">
    <section class="hero">
      <div class="hero-copy">
        <div class="hero-label">THE WORLD IS YOURS</div>
        <h2>A little light for your next big adventure.</h2>
        <p>Pick up where you left off, or build a new world from the ground up.</p>
        <div class="hero-actions">
          <button class="button primary" id="hero-create">${icons.plus} New instance</button>
          <button class="button ghost" id="hero-versions">Browse versions</button>
        </div>
      </div>
      <div class="hero-stat"><div><strong>${state.instances.length.toString().padStart(2, "0")}</strong>INSTANCES</div><div><strong>${state.accounts.length.toString().padStart(2, "0")}</strong>ACCOUNTS</div><div><strong>${account ? "READY" : "SETUP"}</strong>PROFILE</div></div>
    </section>
    ${!account ? `<div class="notice" style="margin-top:15px">Add a Microsoft account for online Minecraft servers, or create an offline profile for servers configured to allow offline-mode connections. Offline profiles do not bypass online authentication.</div>` : ""}
    <div class="section-head"><div><h2>Your instances</h2><p>Separate game folders for versions, loaders, mods, and saves.</p></div><div class="section-actions"><button class="button small" id="import-pack">Import .mrpack</button><button class="button small" id="create-instance">${icons.plus} Create</button></div></div>
    <div class="instance-grid">${state.instances.length ? state.instances.map(instanceCard).join("") : `<div class="empty-state" style="grid-column:1/-1"><h3>Your library is ready for a first world</h3><p>Create an instance and Lumen will download the selected Minecraft version into its library.</p><button class="button primary small" id="empty-create">${icons.plus} Create an instance</button></div>`}</div>
    ${state.activeInstall ? `<div class="panel panel-pad" style="margin-top:16px"><div class="section-head" style="margin:0 0 9px"><div><h2>${esc(state.activeInstall.message || "Installing Minecraft")}</h2><p>${esc(state.activeInstall.versionId || "")}</p></div><span class="mono muted">${state.activeInstall.progress || 0}%</span></div><div class="progress-track"><div class="progress-fill" style="width:${state.activeInstall.progress || 0}%"></div></div></div>` : ""}
  </div>`;
}

function instancesPage() {
  return `<div class="content">
    <div class="page-heading"><div><p class="eyebrow">YOUR LIBRARY</p><h1>Instances</h1><p class="subtitle">Every instance keeps its own version, mods, worlds, resource packs, memory allocation, and launch settings.</p></div><div class="section-actions"><button class="button" id="import-zip">Import ZIP</button><button class="button primary" id="instances-create">${icons.plus} New instance</button></div></div>
    <div class="instance-grid">${state.instances.length ? state.instances.map(instanceCard).join("") : `<div class="empty-state" style="grid-column:1/-1"><h3>No instances yet</h3><p>Create a clean install or import a saved instance to get started.</p><button class="button primary small" id="instances-create-empty">${icons.plus} Create instance</button></div>`}</div>
    <div id="instance-detail"></div>
  </div>`;
}

function modsPage() {
  const instance = selectedInstance();
  return `<div class="content">
    <div class="page-heading"><div><p class="eyebrow">MODRINTH · API V2</p><h1>Mod browser</h1><p class="subtitle">Search compatible mods for the selected Minecraft version and loader. Required dependencies are installed automatically.</p></div></div>
    ${!instance ? `<div class="empty-state"><h3>Select an instance first</h3><p>Create an instance before searching for compatible mods.</p><button class="button small" data-page="instances">Go to instances</button></div>` : `
      <div class="panel panel-pad">
        <div class="section-head" style="margin:0 0 14px"><div><h2>Target instance</h2><p>${esc(instance.name)} · ${esc(instance.gameVersionId)} · ${esc(instance.loader || "Vanilla")}</p></div><select class="select" id="mod-instance" style="width:210px">${state.instances.map(item => `<option value="${esc(item.id)}" ${item.id === instance.id ? "selected" : ""}>${esc(item.name)}</option>`).join("")}</select></div>
        <div class="search-row"><input id="mod-query" class="input" placeholder="Search mods by name or feature" autocomplete="off"><button class="button primary" id="mod-search">Search</button></div>
        <div class="mod-list" id="mod-results">${state.modResults.length ? state.modResults.map(modResult).join("") : `<div class="empty-state"><h3>Find your next favorite mod</h3><p>Search the Modrinth catalog, filtered for ${esc(instance.gameVersionId)}${instance.loader ? ` and ${esc(instance.loader)}` : ""}.</p></div>`}</div>
      </div>
      <div class="section-head"><div><h2>Installed files</h2><p>Disable a mod without deleting it from the instance.</p></div><button class="button small" id="refresh-mods">Refresh</button></div>
      <div class="panel panel-pad"><div class="mod-installed" id="installed-mods">${state.installedMods.length ? state.installedMods.map(modRow).join("") : `<div class="muted" style="font-size:10px">No mod files found in this instance.</div>`}</div></div>
    `}
  </div>`;
}

function modResult(mod) {
  const icon = mod.iconUrl ? `<img src="${esc(mod.iconUrl)}" alt="" onerror="this.remove()">` : "M";
  return `<article class="mod-result">${`<div class="mod-icon">${icon}</div>`}<div class="mod-copy"><div class="mod-title">${esc(mod.title)}</div><div class="mod-desc">${esc(mod.description)}</div><div class="mod-meta"><span>by ${esc(mod.author)}</span><span>${Number(mod.downloads || 0).toLocaleString()} downloads</span></div></div><button class="button small" data-install-mod="${esc(mod.projectId)}">Install</button></article>`;
}

function modRow(filename) {
  const disabled = filename.endsWith(".disabled");
  return `<div class="mod-row"><span class="instance-cube" style="width:26px;height:26px;border-radius:6px">${icons.cube}</span><span class="mod-row-name">${esc(filename)}</span><span class="tag ${disabled ? "" : "green"}">${disabled ? "Disabled" : "Enabled"}</span><button class="button small" data-toggle-mod="${esc(filename)}" data-enabled="${disabled}">${disabled ? "Enable" : "Disable"}</button></div>`;
}

function activityPage() {
  return `<div class="content">
    <div class="page-heading"><div><p class="eyebrow">DIAGNOSTICS</p><h1>Game activity</h1><p class="subtitle">Live output from launched instances and crash reports stored in your launcher data directory.</p></div><button class="button small" id="clear-log">Clear console</button></div>
    <div class="section-head" style="margin-top:0"><div><h2>Live console</h2><p>Console output appears while a game process is running.</p></div><span class="tag">${state.logs.length ? `${state.logs.length} LINES` : "IDLE"}</span></div>
    <div class="log-console" id="log-console">${state.logs.length ? state.logs.map(log => `<span class="log-line ${log.stream === "stderr" ? "error" : ""}">${esc(log.line)}</span>`).join("") : "No game process output yet. Launch an instance to stream its log here."}</div>
     <div class="section-head"><div><h2>Crash reports</h2><p>Reports are read from each instance's Minecraft crash-reports folder.</p></div><button class="button small" id="refresh-crashes">Refresh reports</button></div>
    <div class="panel panel-pad" id="crash-list">${state.crashReports.length ? `<table class="table"><thead><tr><th>Report</th><th>Modified</th><th></th></tr></thead><tbody>${state.crashReports.map(report => `<tr><td><span class="mono">${esc(report.filename)}</span><div class="muted" style="margin-top:5px">${esc(report.preview)}</div><div class="muted" style="margin-top:3px">${esc(state.instances.find(item => item.id === report.instanceId)?.name || "Instance")}</div></td><td>${formatDate(report.modifiedAt)}</td><td><button class="button small" data-read-crash="${esc(report.filename)}" data-crash-instance="${esc(report.instanceId)}">Read</button></td></tr>`).join("")}</tbody></table>` : `<div class="muted" style="font-size:10px">No crash reports found.</div>`}</div>
  </div>`;
}

function settingsPage() {
  return `<div class="content">
    <div class="page-heading"><div><p class="eyebrow">PREFERENCES</p><h1>Settings</h1><p class="subtitle">Tune runtime memory, Java selection, language, and quick-join behavior.</p></div></div>
    <div class="settings-layout">
      <section class="panel panel-pad">
        <div class="section-head" style="margin:0 0 3px"><div><h2>Game defaults</h2><p>These apply to new instances unless overridden.</p></div></div>
        <div class="settings-row"><div><div class="settings-title">Memory allocation</div><div class="settings-desc">Maximum heap memory for new instances. Keep some memory available for your operating system.</div></div><div class="ram-row"><input id="ram-slider" type="range" min="1024" max="16384" step="512" value="${Number(state.settings.ramMb) || 4096}"><span class="ram-value" id="ram-value">${((Number(state.settings.ramMb) || 4096) / 1024).toFixed(1)} GB</span></div></div>
        <div class="settings-row"><div><div class="settings-title">Custom Java executable</div><div class="settings-desc">Leave blank to download a compatible Eclipse Temurin runtime automatically.</div></div><input id="java-path" class="input" placeholder="Path to java or java.exe" value="${esc(state.settings.javaPath || "")}"></div>
        <div class="settings-row"><div><div class="settings-title">Quick-join server</div><div class="settings-desc">Optional host name or host:port. Leave blank to launch to the main menu.</div></div><input id="server-address" class="input" placeholder="play.example.net:25565" value="${esc(state.settings.serverAddress || "")}"></div>
        <div class="settings-row"><div><div class="settings-title">Interface language</div><div class="settings-desc">Arabic uses a right-to-left layout. Game language is controlled inside Minecraft.</div></div><select id="language" class="select"><option value="en" ${state.settings.language === "en" ? "selected" : ""}>English</option><option value="ar" ${state.settings.language === "ar" ? "selected" : ""}>العربية · Arabic</option></select></div>
        <div class="modal-actions"><button class="button primary" id="save-settings">Save preferences</button></div>
      </section>
      <section class="panel panel-pad">
        <div class="section-head" style="margin:0 0 14px"><div><h2>Accounts</h2><p>Microsoft credentials stay in your operating system keyring.</p></div><button class="button small" id="manage-accounts">Manage</button></div>
        ${state.accounts.length ? `<div class="account-list">${state.accounts.map(accountRow).join("")}</div>` : `<div class="empty-state"><h3>No accounts yet</h3><p>Add a Microsoft account or a profile for offline-mode servers.</p></div>`}
        <div class="spacer"></div>
        <div class="notice">Offline profiles are only for offline-mode servers. They do not provide Minecraft ownership or grant access to authenticated online servers.</div>
        <div class="spacer"></div>
        <div class="field"><label>Launcher data location</label><div class="input mono" id="data-directory" style="word-break:break-all;height:auto">${esc(window.launcherDataDir || "Loading…")}</div></div>
      </section>
    </div>
  </div>`;
}

function accountRow(account) {
  return `<div class="account-row">${avatar(account, 32)}<div class="account-row-copy"><div class="account-row-name">${esc(account.username)} ${account.active ? `<span class="tag green">Active</span>` : ""}</div><div class="account-row-type">${account.accountType === "microsoft" ? "Microsoft · Java ownership checked" : "Offline · offline-mode servers only"}</div></div><div class="account-actions">${account.active ? "" : `<button class="button small" data-activate-account="${esc(account.id)}">Use</button>`}<button class="button danger small" data-remove-account="${esc(account.id)}">Remove</button></div></div>`;
}

function render() {
  const page = ({
    overview: overviewPage,
    instances: instancesPage,
    mods: modsPage,
    activity: activityPage,
    settings: settingsPage,
  })[state.page] || overviewPage;
  document.documentElement.lang = state.settings.language || "en";
  document.documentElement.dir = state.settings.language === "ar" ? "rtl" : "ltr";
  document.getElementById("app").innerHTML = `<div class="shell">${sidebar()}<main class="main">${header()}${page()}</main></div>`;
  applyArabic(document.getElementById("app"));
  wireEvents();
}

function openModal(title, subtitle, body, options = {}) {
  const root = document.getElementById("modal-root");
  root.innerHTML = `<div class="modal-backdrop" data-dismiss-modal><section class="modal ${options.wide ? "wide" : ""}" role="dialog" aria-modal="true" aria-label="${esc(title)}">
    <div class="modal-head"><div><h2>${esc(title)}</h2><p>${esc(subtitle || "")}</p></div><button class="modal-close" id="modal-close" aria-label="Close">×</button></div>
    <div class="modal-body">${body}</div>
  </section></div>`;
  applyArabic(root);
  root.querySelector("#modal-close").onclick = closeModal;
  root.querySelector(".modal-backdrop").addEventListener("click", (event) => {
    if (event.target === event.currentTarget) closeModal();
  });
}

function closeModal() {
  document.getElementById("modal-root").innerHTML = "";
}

function showAccountChooser() {
  openModal("Add an account", "Choose how this launcher should identify you in-game.", `<div class="account-choice">
    <button class="account-option" id="choose-microsoft"><span class="account-option-icon">M</span><span><strong>Microsoft account</strong><small>For Minecraft Java owners and authenticated servers.</small></span></button>
    <button class="account-option" id="choose-offline"><span class="account-option-icon">O</span><span><strong>Offline profile</strong><small>Username-only profile for servers configured for offline mode.</small></span></button>
  </div><div class="spacer"></div><div class="notice">Offline profiles do not skip Microsoft sign-in or ownership checks for premium online access.</div>`);
  document.getElementById("choose-microsoft").onclick = startMicrosoftFlow;
  document.getElementById("choose-offline").onclick = showOfflineForm;
}

function showOfflineForm() {
  openModal("Create offline profile", "Choose a Java username for offline-mode servers.", `<div class="field"><label for="offline-name">Username</label><input id="offline-name" class="input" maxlength="16" placeholder="Builder_01" autocomplete="off"><small>1–16 letters, numbers, or underscores.</small></div><div id="offline-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="offline-cancel">Back</button><button class="button primary" id="offline-submit">Create profile</button></div>`);
  document.getElementById("offline-cancel").onclick = showAccountChooser;
  document.getElementById("offline-submit").onclick = async () => {
    const button = document.getElementById("offline-submit");
    const error = document.getElementById("offline-error");
    setBusy(button, true, "Creating…");
    try {
      await invoke("add_offline_account", { username: document.getElementById("offline-name").value });
      closeModal();
      await loadState();
      toast("Offline profile added.");
    } catch (err) {
      error.textContent = String(err);
      error.classList.remove("hidden");
    } finally { setBusy(button, false); }
  };
}

async function startMicrosoftFlow() {
  try {
    const device = await invoke("begin_microsoft_login");
    openModal("Sign in with Microsoft", "Complete sign-in in your browser. Lumen will verify Java ownership before saving the account.", `<div class="notice green">Open the Microsoft device sign-in page, enter this code, and approve the request.</div><div class="code-block" id="device-code">${esc(device.userCode)}</div><p style="text-align:center"><a class="inline-link" href="${esc(device.verificationUri)}" target="_blank" rel="noreferrer">Open ${esc(device.verificationUri)}</a></p><div id="device-status" class="muted" style="font-size:10px;text-align:center">Waiting for approval…</div><div class="modal-actions"><button class="button" id="device-cancel">Cancel</button></div>`);
    document.getElementById("device-cancel").onclick = closeModal;
    const interval = Math.max(2, Number(device.interval) || 5) * 1000;
    const expiresAt = Date.now() + Number(device.expiresIn) * 1000;
    let stopped = false;
    document.getElementById("device-cancel").onclick = () => { stopped = true; closeModal(); };
    const poll = async () => {
      if (stopped || Date.now() > expiresAt || !document.getElementById("device-status")) return;
      try {
        await invoke("finish_microsoft_login", { deviceCode: device.deviceCode });
        closeModal();
        await loadState();
        toast("Microsoft account connected and ownership verified.");
      } catch (err) {
        const message = String(err);
        const status = document.getElementById("device-status");
        if (message.includes("still waiting") || message.includes("poll more slowly")) {
          if (status) status.textContent = message.includes("slowly") ? "Microsoft asked us to wait a little longer…" : "Waiting for approval…";
          if (!stopped) setTimeout(poll, interval + (message.includes("slowly") ? 3000 : 0));
          return;
        }
        if (status) status.textContent = message;
      }
    };
    setTimeout(poll, interval);
  } catch (err) {
    openModal("Microsoft sign-in unavailable", "The official sign-in flow could not be started.", `<div class="notice">${esc(err)}</div><div class="modal-actions"><button class="button" id="account-back">Back</button></div>`);
    document.getElementById("account-back").onclick = showAccountChooser;
  }
}

async function showAccountManager() {
  openModal("Manage accounts", "Select the account used for launches, or remove profiles you no longer need.", `<div class="account-list">${state.accounts.length ? state.accounts.map(accountRow).join("") : `<div class="empty-state"><h3>No accounts connected</h3><p>Add an account to enable game launches.</p></div>`}</div><div class="modal-actions"><button class="button" id="account-manage-close">Close</button><button class="button primary" id="account-manage-add">Add account</button></div>`, { wide: true });
  document.getElementById("account-manage-close").onclick = closeModal;
  document.getElementById("account-manage-add").onclick = showAccountChooser;
  wireAccountRows();
}

async function showCreateInstance() {
  if (!state.versions.length) {
    try { await loadVersions(); }
    catch (err) { toast(String(err), true); return; }
  }
  const options = state.versions.slice(0, 100).map(version => `<option value="${esc(version.id)}">${esc(version.id)} · ${esc(version.kind)}</option>`).join("");
  openModal("Create instance", "Choose an official Minecraft version. The vanilla game files will be installed before the instance is added.", `<div class="field"><label for="instance-name">Instance name</label><input id="instance-name" class="input" maxlength="48" placeholder="Vanilla survival" value="New instance"></div><div class="field"><label for="instance-version">Minecraft version</label><select id="instance-version" class="select">${options || `<option value="">No versions loaded</option>`}</select><small>Choose a release or snapshot from Mojang’s version manifest.</small></div><div id="instance-create-error" class="notice hidden"></div><div id="instance-progress" class="hidden"><div class="progress-track"><div class="progress-fill"></div></div><div class="muted" style="font-size:9px;text-align:center">Preparing game files…</div></div><div class="modal-actions"><button class="button" id="create-cancel">Cancel</button><button class="button primary" id="create-submit">Install instance</button></div>`);
  document.getElementById("create-cancel").onclick = closeModal;
  document.getElementById("create-submit").onclick = async () => {
    const button = document.getElementById("create-submit");
    const error = document.getElementById("instance-create-error");
    const progress = document.getElementById("instance-progress");
    const name = document.getElementById("instance-name").value;
    const versionId = document.getElementById("instance-version").value;
    setBusy(button, true, "Installing…");
    progress.classList.remove("hidden");
    try {
      await invoke("create_instance", { name, versionId });
      closeModal();
      await loadState();
      toast("Instance installed and ready.");
    } catch (err) {
      error.textContent = String(err);
      error.classList.remove("hidden");
      setBusy(button, false);
    }
  };
}

async function showVersions() {
  try {
    if (!state.versions.length) state.versions = await invoke("list_versions");
    openModal("Minecraft versions", "Choose a version when creating an instance. Releases and snapshots are read from the official manifest.", `<div class="field"><label for="version-filter">Filter versions</label><input id="version-filter" class="input" placeholder="Search by version ID"></div><div class="panel" style="max-height:390px;overflow:auto" id="version-list">${versionRows(state.versions.slice(0, 60))}</div><div class="modal-actions"><button class="button" id="versions-close">Close</button><button class="button primary" id="versions-create">Create instance</button></div>`, { wide: true });
    document.getElementById("version-filter").oninput = (event) => {
      const query = event.target.value.toLowerCase();
      document.getElementById("version-list").innerHTML = versionRows(state.versions.filter(version => version.id.toLowerCase().includes(query)).slice(0, 80));
    };
    document.getElementById("versions-close").onclick = closeModal;
    document.getElementById("versions-create").onclick = showCreateInstance;
  } catch (err) { toast(String(err), true); }
}

function versionRows(versions) {
  return `<table class="table"><thead><tr><th>Version</th><th>Type</th><th>Released</th></tr></thead><tbody>${versions.map(version => `<tr><td class="mono">${esc(version.id)}</td><td><span class="tag ${version.kind === "release" ? "green" : ""}">${esc(version.kind)}</span></td><td>${esc((version.releaseTime || "").slice(0, 10))}</td></tr>`).join("")}</tbody></table>`;
}

function instanceOptions(instanceId) {
  const instance = state.instances.find(item => item.id === instanceId);
  if (!instance) return;
  openModal("Instance options", "Manage this instance’s settings and files.", `<div class="account-choice">
    <button class="account-option" id="instance-clone"><span class="account-option-icon">C</span><span><strong>Clone instance</strong><small>Copy its settings, mods, and game files.</small></span></button>
    <button class="account-option" id="instance-export"><span class="account-option-icon">↗</span><span><strong>Export ZIP</strong><small>Save an archive to a local path.</small></span></button>
    <button class="account-option" id="instance-loader"><span class="account-option-icon">L</span><span><strong>Install a mod loader</strong><small>Fabric, Quilt, Forge, or NeoForge.</small></span></button>
    <button class="account-option" id="instance-configure"><span class="account-option-icon">G</span><span><strong>Configure instance</strong><small>Memory, Java executable, and JVM arguments.</small></span></button>
    <button class="account-option" id="instance-delete"><span class="account-option-icon" style="color:var(--red)">×</span><span><strong>Delete instance</strong><small>Remove this instance and its local world data.</small></span></button>
  </div>`);
  document.getElementById("instance-clone").onclick = () => cloneInstance(instance);
  document.getElementById("instance-export").onclick = () => exportInstance(instance);
  document.getElementById("instance-loader").onclick = () => installLoader(instance);
  document.getElementById("instance-configure").onclick = () => configureInstance(instance);
  document.getElementById("instance-delete").onclick = () => deleteInstance(instance);
}

function cloneInstance(instance) {
  openModal("Clone instance", "The original stays unchanged.", `<div class="field"><label for="clone-name">New instance name</label><input class="input" id="clone-name" maxlength="48" value="${esc(`${instance.name} copy`)}"></div><div id="clone-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="clone-cancel">Cancel</button><button class="button primary" id="clone-submit">Clone files</button></div>`);
  document.getElementById("clone-cancel").onclick = closeModal;
  document.getElementById("clone-submit").onclick = async () => {
    try {
      await invoke("clone_instance", { instanceId: instance.id, name: document.getElementById("clone-name").value });
      closeModal(); await loadState(); toast("Instance cloned.");
    } catch (err) { const target = document.getElementById("clone-error"); target.textContent = String(err); target.classList.remove("hidden"); }
  };
}

function exportInstance(instance) {
  openModal("Export instance", "Enter a local destination path. For example, a path ending in .zip.", `<div class="field"><label for="export-path">ZIP destination</label><input class="input" id="export-path" placeholder="/home/user/Downloads/world.zip"></div><div id="export-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="export-cancel">Cancel</button><button class="button primary" id="export-submit">Export ZIP</button></div>`);
  document.getElementById("export-cancel").onclick = closeModal;
  document.getElementById("export-submit").onclick = async () => {
    try {
      await invoke("export_instance", { instanceId: instance.id, destination: document.getElementById("export-path").value });
      closeModal(); toast("Instance ZIP exported.");
    } catch (err) { const target = document.getElementById("export-error"); target.textContent = String(err); target.classList.remove("hidden"); }
  };
}

function installLoader(instance) {
  openModal("Install a mod loader", "Install a compatible loader into this instance. This changes the active launch profile.", `<div class="field"><label for="loader-kind">Loader</label><select id="loader-kind" class="select"><option value="fabric">Fabric</option><option value="quilt">Quilt</option><option value="forge">Forge</option><option value="neoforge">NeoForge</option></select></div><div class="field"><label for="loader-version">Loader version (optional)</label><input id="loader-version" class="input" placeholder="Leave blank for latest compatible build"></div><div class="notice">Forge and NeoForge installers are launched locally and may take several minutes.</div><div id="loader-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="loader-cancel">Cancel</button><button class="button primary" id="loader-submit">Install loader</button></div>`);
  document.getElementById("loader-cancel").onclick = closeModal;
  document.getElementById("loader-submit").onclick = async () => {
    const button = document.getElementById("loader-submit");
    setBusy(button, true, "Installing…");
    try {
      await invoke("install_loader", { instanceId: instance.id, loader: document.getElementById("loader-kind").value, loaderVersion: document.getElementById("loader-version").value || null });
      closeModal(); await loadState(); toast("Mod loader installed.");
    } catch (err) {
      const target = document.getElementById("loader-error"); target.textContent = String(err); target.classList.remove("hidden");
      setBusy(button, false);
    }
  };
}

function configureInstance(instance) {
  openModal("Configure instance", "Adjust the runtime and JVM settings for this instance.", `<div class="field"><label for="instance-ram">Memory (MB)</label><input class="input" type="number" min="1024" max="65536" step="512" id="instance-ram" value="${Number(instance.ramMb) || 4096}"></div><div class="field"><label for="instance-java">Custom Java executable</label><input class="input" id="instance-java" placeholder="Blank uses the managed runtime" value="${esc(instance.javaPath || "")}"></div><div class="field"><label for="instance-jvm">Additional JVM arguments</label><textarea class="textarea" id="instance-jvm" placeholder="-XX:+UseG1GC">${esc((instance.jvmArgs || []).join("\n"))}</textarea><small>One argument per line; avoid overriding classpath or Minecraft-owned flags.</small></div><div id="configure-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="configure-cancel">Cancel</button><button class="button primary" id="configure-submit">Save settings</button></div>`);
  document.getElementById("configure-cancel").onclick = closeModal;
  document.getElementById("configure-submit").onclick = async () => {
    try {
      const updated = { ...instance, ramMb: Number(document.getElementById("instance-ram").value), javaPath: document.getElementById("instance-java").value || null, jvmArgs: document.getElementById("instance-jvm").value.split("\n").map(item => item.trim()).filter(Boolean) };
      await invoke("update_instance", { instance: updated });
      closeModal(); await loadState(); toast("Instance settings saved.");
    } catch (err) { const target = document.getElementById("configure-error"); target.textContent = String(err); target.classList.remove("hidden"); }
  };
}

function deleteInstance(instance) {
  openModal("Delete this instance?", `This permanently removes ${instance.name} and its local worlds, mods, resource packs, and settings.`, `<div class="notice">Export a ZIP first if you want to keep the world or files.</div><div id="delete-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="delete-cancel">Keep instance</button><button class="button danger" id="delete-submit">Delete instance</button></div>`);
  document.getElementById("delete-cancel").onclick = closeModal;
  document.getElementById("delete-submit").onclick = async () => {
    try { await invoke("delete_instance", { instanceId: instance.id }); closeModal(); await loadState(); toast("Instance deleted."); }
    catch (err) { const target = document.getElementById("delete-error"); target.textContent = String(err); target.classList.remove("hidden"); }
  };
}

function showImportPack() {
  openModal("Import Modrinth modpack", "Enter the local path to a .mrpack file. Its Minecraft version, loader, overrides, and downloadable files will be installed.", `<div class="field"><label for="mrpack-path">.mrpack file path</label><input id="mrpack-path" class="input" placeholder="/home/user/Downloads/modpack.mrpack"></div><div class="field"><label for="mrpack-name">Instance name (optional)</label><input id="mrpack-name" class="input" placeholder="Use the pack name"></div><div id="mrpack-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="mrpack-cancel">Cancel</button><button class="button primary" id="mrpack-submit">Import pack</button></div>`);
  document.getElementById("mrpack-cancel").onclick = closeModal;
  document.getElementById("mrpack-submit").onclick = async () => {
    const button = document.getElementById("mrpack-submit"); setBusy(button, true, "Installing pack…");
    try {
      await invoke("import_modpack", { sourcePath: document.getElementById("mrpack-path").value, name: document.getElementById("mrpack-name").value || null });
      closeModal(); await loadState(); toast("Modrinth modpack imported.");
    } catch (err) { const target = document.getElementById("mrpack-error"); target.textContent = String(err); target.classList.remove("hidden"); setBusy(button, false); }
  };
}

function showImportZip() {
  if (!state.versions.length) { loadVersions().then(showImportZip); return; }
  const version = state.versions.find(item => item.kind === "release")?.id || state.versions[0]?.id || "";
  openModal("Import instance ZIP", "Import a ZIP produced by Lumen Launcher. For a generic archive, choose its Minecraft version.", `<div class="field"><label for="zip-path">ZIP file path</label><input id="zip-path" class="input" placeholder="/home/user/Downloads/instance.zip"></div><div class="field"><label for="zip-name">Instance name</label><input id="zip-name" class="input" placeholder="Imported instance"></div><div class="field"><label for="zip-version">Fallback Minecraft version</label><select id="zip-version" class="select">${state.versions.slice(0,100).map(item => `<option value="${esc(item.id)}" ${item.id === version ? "selected" : ""}>${esc(item.id)}</option>`).join("")}</select></div><div id="zip-error" class="notice hidden"></div><div class="modal-actions"><button class="button" id="zip-cancel">Cancel</button><button class="button primary" id="zip-submit">Import ZIP</button></div>`);
  document.getElementById("zip-cancel").onclick = closeModal;
  document.getElementById("zip-submit").onclick = async () => {
    try {
      await invoke("import_instance", { sourcePath: document.getElementById("zip-path").value, name: document.getElementById("zip-name").value, versionId: document.getElementById("zip-version").value });
      closeModal(); await loadState(); toast("Instance ZIP imported.");
    } catch (err) { const target = document.getElementById("zip-error"); target.textContent = String(err); target.classList.remove("hidden"); }
  };
}

function wireEvents() {
  document.querySelectorAll("[data-page]").forEach(button => button.onclick = () => {
    state.page = button.dataset.page;
    if (state.page === "mods") refreshMods();
    if (state.page === "activity") refreshCrashes();
    render();
  });
  document.getElementById("sidebar-add-account")?.addEventListener("click", showAccountManager);
  document.getElementById("account-switch")?.addEventListener("click", showAccountManager);
  document.getElementById("hero-create")?.addEventListener("click", showCreateInstance);
  document.getElementById("empty-create")?.addEventListener("click", showCreateInstance);
  document.getElementById("create-instance")?.addEventListener("click", showCreateInstance);
  document.getElementById("instances-create")?.addEventListener("click", showCreateInstance);
  document.getElementById("instances-create-empty")?.addEventListener("click", showCreateInstance);
  document.getElementById("hero-versions")?.addEventListener("click", showVersions);
  document.getElementById("import-pack")?.addEventListener("click", showImportPack);
  document.getElementById("import-zip")?.addEventListener("click", showImportZip);
  document.querySelectorAll("[data-instance-card]").forEach(card => card.addEventListener("click", event => {
    if (event.target.closest("button")) return;
    state.selectedInstance = card.dataset.instanceCard;
    render();
  }));
  document.querySelectorAll("[data-instance-menu]").forEach(button => button.onclick = () => instanceOptions(button.dataset.instanceMenu));
  document.querySelectorAll("[data-launch]").forEach(button => button.onclick = () => runInstance(button.dataset.launch, button));
  document.getElementById("mod-instance")?.addEventListener("change", async (event) => {
    state.selectedInstance = event.target.value;
    state.modResults = [];
    await refreshMods();
    render();
  });
  document.getElementById("mod-search")?.addEventListener("click", searchMods);
  document.getElementById("mod-query")?.addEventListener("keydown", event => { if (event.key === "Enter") searchMods(); });
  document.querySelectorAll("[data-install-mod]").forEach(button => button.onclick = () => installMod(button.dataset.installMod, button));
  document.querySelectorAll("[data-toggle-mod]").forEach(button => button.onclick = () => toggleMod(button));
  document.getElementById("refresh-mods")?.addEventListener("click", refreshMods);
  document.getElementById("clear-log")?.addEventListener("click", () => { state.logs = []; render(); });
  document.getElementById("refresh-crashes")?.addEventListener("click", refreshCrashes);
  document.querySelectorAll("[data-read-crash]").forEach(button => button.onclick = () => readCrash(button.dataset.crashInstance, button.dataset.readCrash));
  document.getElementById("manage-accounts")?.addEventListener("click", showAccountManager);
  document.getElementById("save-settings")?.addEventListener("click", saveSettings);
  document.getElementById("ram-slider")?.addEventListener("input", event => {
    const value = Number(event.target.value) / 1024;
    document.getElementById("ram-value").textContent = `${value.toFixed(1)} GB`;
  });
}

function wireAccountRows() {
  document.querySelectorAll("[data-activate-account]").forEach(button => button.onclick = async () => {
    try { await invoke("set_active_account", { accountId: button.dataset.activateAccount }); await loadState(); showAccountManager(); }
    catch (err) { toast(String(err), true); }
  });
  document.querySelectorAll("[data-remove-account]").forEach(button => button.onclick = async () => {
    if (!confirm("Remove this account and its saved keyring credentials?")) return;
    try { await invoke("remove_account", { accountId: button.dataset.removeAccount }); await loadState(); showAccountManager(); }
    catch (err) { toast(String(err), true); }
  });
}

async function runInstance(instanceId, button) {
  const account = activeAccount();
  if (!account) { showAccountChooser(); return; }
  setBusy(button, true, "Starting…");
  try {
    state.selectedInstance = instanceId;
    const pid = await invoke("launch_instance", { instanceId });
    state.page = "activity";
    await refreshCrashes();
    render();
    toast(`Minecraft started${pid ? ` · process ${pid}` : ""}.`);
  } catch (err) { toast(String(err), true); }
  finally { if (button.isConnected) setBusy(button, false); }
}

async function loadVersions() {
  state.versions = await invoke("list_versions");
  return state.versions;
}

async function searchMods() {
  const instance = selectedInstance();
  if (!instance) return;
  const button = document.getElementById("mod-search");
  setBusy(button, true, "Searching…");
  try {
    state.modResults = await invoke("search_modrinth", {
      query: document.getElementById("mod-query").value,
      gameVersion: instance.gameVersionId,
      loader: instance.loader || null,
    });
    render();
  } catch (err) { toast(String(err), true); }
  finally { const current = document.getElementById("mod-search"); if (current) setBusy(current, false); }
}

async function installMod(projectId, button) {
  const instance = selectedInstance();
  if (!instance) return;
  setBusy(button, true, "Installing…");
  try {
    const files = await invoke("install_mod", { instanceId: instance.id, projectId });
    toast(`Installed ${files.length} mod file${files.length === 1 ? "" : "s"} including required dependencies.`);
    await refreshMods();
  } catch (err) { toast(String(err), true); }
  finally { if (button.isConnected) setBusy(button, false); }
}

async function refreshMods() {
  const instance = selectedInstance();
  if (!instance) { state.installedMods = []; return; }
  try { state.installedMods = await invoke("list_mods", { instanceId: instance.id }); }
  catch (err) { state.installedMods = []; toast(String(err), true); }
  if (state.page === "mods") render();
}

async function toggleMod(button) {
  const instance = selectedInstance();
  if (!instance) return;
  try {
    await invoke("set_mod_enabled", { instanceId: instance.id, filename: button.dataset.toggleMod, enabled: button.dataset.enabled === "true" });
    await refreshMods();
    toast("Mod state updated.");
  } catch (err) { toast(String(err), true); }
}

async function refreshCrashes() {
  try { state.crashReports = await invoke("list_crash_reports"); }
  catch (err) { toast(String(err), true); }
  if (state.page === "activity") render();
}

async function readCrash(instanceId, filename) {
  try {
    const content = await invoke("read_crash_report", { instanceId, filename });
    openModal(filename, "Crash report", `<pre class="log-console" style="max-height:60vh">${esc(content)}</pre><div class="modal-actions"><button class="button" id="crash-close">Close</button></div>`, { wide: true });
    document.getElementById("crash-close").onclick = closeModal;
  } catch (err) { toast(String(err), true); }
}

async function saveSettings() {
  const button = document.getElementById("save-settings");
  setBusy(button, true, "Saving…");
  try {
    state.settings = await invoke("save_settings", {
      settings: {
        language: document.getElementById("language").value,
        ramMb: Number(document.getElementById("ram-slider").value),
        javaPath: document.getElementById("java-path").value || null,
        serverAddress: document.getElementById("server-address").value.trim(),
      },
    });
    render();
    toast("Preferences saved.");
  } catch (err) { toast(String(err), true); }
  finally { const current = document.getElementById("save-settings"); if (current) setBusy(current, false); }
}

async function loadState() {
  if (!invoke) {
    document.getElementById("app").innerHTML = `<main class="boot"><div class="brand-mark"><span></span><span></span><span></span><span></span></div><p>This launcher needs to run in its Tauri desktop window.</p></main>`;
    return;
  }
  try {
    const data = await invoke("get_launcher_state");
    Object.assign(state, data);
    window.launcherDataDir = data.dataDirectory;
    if (!state.selectedInstance && state.instances.length) state.selectedInstance = state.instances[0].id;
    render();
  } catch (err) {
    document.getElementById("app").innerHTML = `<main class="boot"><h2>Launcher could not load</h2><p>${esc(err)}</p></main>`;
  }
}

async function setupListeners() {
  if (!listen) return;
  await listen("install-progress", event => {
    state.activeInstall = event.payload;
    if (state.page === "overview") render();
  });
  await listen("game-log", event => {
    state.logs.push(event.payload);
    if (state.logs.length > 1200) state.logs.splice(0, state.logs.length - 1200);
    if (state.page === "activity") {
      const consoleElement = document.getElementById("log-console");
      if (consoleElement) {
        const line = document.createElement("span");
        line.className = `log-line ${event.payload.stream === "stderr" ? "error" : ""}`;
        line.textContent = event.payload.line;
        consoleElement.append(line);
        consoleElement.scrollTop = consoleElement.scrollHeight;
      }
    }
  });
  await listen("game-exit", event => {
    toast(`Game process ended: ${event.payload.status}`);
  });
}

async function init() {
  await setupListeners();
  await loadState();
}

init();
