const STORAGE_KEY = "yunxi-web-redesign-chat";

const nodes = {
  dock: document.querySelector("#yunxi-dock"),
  navItems: Array.from(document.querySelectorAll("[data-nav]")),
  views: Array.from(document.querySelectorAll("[data-view]")),
  runtimeStrip: document.querySelector("#runtime-strip"),
  chatLog: document.querySelector("#chat-log"),
  composer: document.querySelector("#composer"),
  prompt: document.querySelector("#prompt"),
  send: document.querySelector("#send-button"),
  memoryField: document.querySelector("#memory-field"),
  memoryReveal: document.querySelector("#memory-reveal"),
  memorySpread: document.querySelector("#memory-spread"),
  memoryClose: document.querySelector("#memory-close"),
  memoryDetailKind: document.querySelector("#memory-detail-kind"),
  memoryDetailTitle: document.querySelector("#memory-detail-title"),
  memoryDetailContent: document.querySelector("#memory-detail-content"),
  memoryDetailMeta: document.querySelector("#memory-detail-meta"),
  personaDeck: document.querySelector("#persona-deck"),
  personaPrev: document.querySelector("#persona-prev"),
  personaNext: document.querySelector("#persona-next"),
  personaDetail: document.querySelector("#persona-detail"),
  personaDetailKicker: document.querySelector("#persona-detail-kicker"),
  personaDetailTitle: document.querySelector("#persona-detail-title"),
  personaDetailSummary: document.querySelector("#persona-detail-summary"),
  personaDetailContent: document.querySelector("#persona-detail-content"),
  personaDetailClose: document.querySelector("#persona-detail-close"),
};

const state = {
  messages: loadMessages(),
  status: null,
  memory: null,
  persona: null,
  activePersonaIndex: 0,
  gsapLoader: null,
  viewAnimations: [],
  personaEntrancePlayed: false,
  autoResizeFrame: 0,
  resizeFrame: 0,
  memoryCloseTimer: 0,
  personaCloseTimer: 0,
  lastMemorySource: null,
  lastPersonaSource: null,
  sending: false,
};

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function loadMessages() {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
    return Array.isArray(value) ? value : [];
  } catch {
    return [];
  }
}

function saveMessages() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state.messages.slice(-80)));
  } catch {
    // Local storage is optional for the web console.
  }
}

function text(value, fallback = "") {
  if (value === null || value === undefined) return fallback;
  return String(value);
}

function truncate(value, length = 72) {
  const source = text(value).replace(/\s+/g, " ").trim();
  if (source.length <= length) return source || "空记忆";
  return `${source.slice(0, length - 1)}…`;
}

function setNodeText(node, value) {
  if (node) node.textContent = value;
}

function hash(value) {
  let result = 2166136261;
  for (const char of text(value)) {
    result ^= char.charCodeAt(0);
    result = Math.imul(result, 16777619);
  }
  return result >>> 0;
}

function seeded(seed, salt) {
  let value = (seed + Math.imul(salt + 1, 2654435761)) >>> 0;
  value ^= value << 13;
  value ^= value >>> 17;
  value ^= value << 5;
  return (value >>> 0) / 4294967295;
}

function clampNumber(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function cancelAnimations(animations) {
  for (const animation of animations || []) {
    animation?.cancel?.();
  }
}

function animateElements(targets, options = {}) {
  const elements = Array.from(targets || []).filter(Boolean);
  if (elements.length === 0 || prefersReducedMotion()) return [];
  const {
    y = 8,
    scale = 1,
    duration = 240,
    stagger = 22,
    ease = "cubic-bezier(0.16, 1, 0.3, 1)",
  } = options;

  if (window.gsap) {
    const tween = window.gsap.fromTo(
      elements,
      { y, scale, autoAlpha: 0 },
      {
        y: 0,
        scale: 1,
        autoAlpha: 1,
        duration: duration / 1000,
        stagger: stagger / 1000,
        ease: "power3.out",
        overwrite: "auto",
        onComplete: () => window.gsap.set(elements, { clearProps: "transform,opacity,visibility" }),
      },
    );
    return [
      {
        cancel() {
          tween.kill();
          window.gsap.set(elements, { clearProps: "transform,opacity,visibility" });
        },
      },
    ];
  }

  return elements.map((element, index) => {
    const animation = element.animate(
      [
        { opacity: 0, transform: `translate3d(0, ${y}px, 0) scale(${scale})` },
        { opacity: 1, transform: "translate3d(0, 0, 0) scale(1)" },
      ],
      {
        duration,
        delay: index * stagger,
        easing: ease,
        fill: "both",
      },
    );
    animation.finished.then(() => animation.cancel()).catch(() => {});
    return animation;
  });
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "content-type": "application/json" },
    ...options,
  });
  const body = await response.text();
  const data = body ? JSON.parse(body) : null;
  if (!response.ok) {
    throw new Error(data?.error || `请求失败：${response.status}`);
  }
  return data;
}

function showView(name) {
  const target = ["chat", "memory", "persona"].includes(name) ? name : "chat";
  document.documentElement.dataset.activeView = target;
  document.body.dataset.activeView = target;
  nodes.views.forEach((view) => {
    const active = view.dataset.view === target;
    view.hidden = !active;
    view.classList.toggle("is-active", active);
  });
  nodes.navItems.forEach((item) => {
    const active = item.dataset.nav === target;
    item.classList.toggle("is-active", active);
    if (active) {
      item.setAttribute("aria-current", "page");
    } else {
      item.removeAttribute("aria-current");
    }
  });
  if (target === "memory") loadMemory();
  if (target === "persona") loadPersona();
  if (target === "chat") {
    window.requestAnimationFrame(() => nodes.prompt?.focus());
  }
  window.requestAnimationFrame(() => animateViewEntrance(target));
  const nextHash = target === "chat" ? "" : `#${target}`;
  if (window.location.hash !== nextHash) {
    history.replaceState(null, "", nextHash || window.location.pathname);
  }
}

function animateViewEntrance(name) {
  cancelAnimations(state.viewAnimations);
  state.viewAnimations = [];
  if (prefersReducedMotion() || name === "persona") return;
  const view = nodes.views.find((candidate) => candidate.dataset.view === name);
  if (!view || view.hidden) return;
  const selectors = {
    chat: ".chat-hero > *",
    memory: ".memory-head > *",
  };
  const targets = Array.from(view.querySelectorAll(selectors[name] || ":scope > *"));
  state.viewAnimations = animateElements(targets, { y: 6, duration: 240, stagger: 22 });
}

function setupDockMotion() {
  if (!nodes.dock) return;
  const items = nodes.navItems;
  let centers = [];
  let pointerX = null;
  let frame = 0;
  let measureFrame = 0;
  let controls = null;

  const measure = () => {
    const rects = items.map((item) => item.getBoundingClientRect());
    centers = rects.map((rect) => rect.left + rect.width / 2);
  };

  const apply = () => {
    frame = 0;
    if (pointerX === null) return;
    const values = centers.map((center) => {
      const distance = Math.abs(pointerX - center);
      const strength = Math.max(0, 1 - distance / 106);
      return { scale: 1 + strength * 0.14, y: -strength * 4 };
    });
    values.forEach((value, index) => {
      if (controls) {
        controls[index].scale(value.scale);
        controls[index].y(value.y);
      } else {
        items[index].style.transform = `translate3d(0, ${value.y.toFixed(2)}px, 0) scale(${value.scale.toFixed(4)})`;
      }
    });
  };

  const schedule = () => {
    if (!frame) frame = window.requestAnimationFrame(apply);
  };

  const scheduleMeasure = () => {
    if (measureFrame) return;
    measureFrame = window.requestAnimationFrame(() => {
      measureFrame = 0;
      measure();
      if (pointerX !== null) schedule();
    });
  };

  const reset = () => {
    pointerX = null;
    if (frame) window.cancelAnimationFrame(frame);
    frame = 0;
    items.forEach((item, index) => {
      if (controls) {
        controls[index].scale(1);
        controls[index].y(0);
      } else {
        item.style.removeProperty("transform");
      }
    });
  };

  nodes.dock.addEventListener("pointerenter", measure);
  nodes.dock.addEventListener("pointermove", (event) => {
    pointerX = event.clientX;
    if (centers.length !== items.length) measure();
    schedule();
  });
  nodes.dock.addEventListener("pointerleave", reset);
  window.addEventListener("resize", scheduleMeasure);

  ensureGsap().then((gsap) => {
    if (!gsap || prefersReducedMotion()) return;
    nodes.dock.classList.add("is-gsap-driven");
    controls = items.map((item) => ({
      scale: gsap.quickTo(item, "scale", { duration: 0.16, ease: "power3.out" }),
      y: gsap.quickTo(item, "y", { duration: 0.16, ease: "power3.out" }),
    }));
    if (pointerX !== null) {
      measure();
      schedule();
    }
  });
}

function autoResize() {
  const node = nodes.prompt;
  if (!node) return;
  nodes.send?.classList.toggle("is-ready", node.value.trim().length > 0);
  if (state.autoResizeFrame) return;
  state.autoResizeFrame = window.requestAnimationFrame(() => {
    state.autoResizeFrame = 0;
    node.style.height = "auto";
    node.style.height = `${Math.max(64, Math.min(node.scrollHeight, 208))}px`;
  });
}

function renderMessages() {
  if (!nodes.chatLog) return;
  nodes.chatLog.innerHTML = "";
  for (const message of state.messages) {
    const article = document.createElement("article");
    article.className = `message ${message.role}${message.kind ? ` ${message.kind}` : ""}`;

    const bubble = document.createElement("div");
    bubble.className = "message-bubble";

    const content = document.createElement("p");
    content.textContent = text(message.text);
    bubble.appendChild(content);

    if (message.meta) {
      const meta = document.createElement("div");
      meta.className = "message-meta";
      meta.textContent = message.meta;
      bubble.appendChild(meta);
    }

    article.appendChild(bubble);
    nodes.chatLog.appendChild(article);
  }
  nodes.chatLog.scrollTop = nodes.chatLog.scrollHeight;
}

function pushMessage(role, message, meta, kind) {
  state.messages.push({
    role,
    text: message,
    meta,
    kind,
    ts: Date.now(),
  });
  saveMessages();
  renderMessages();
}

async function loadStatus() {
  try {
    const status = await api("/api/status");
    state.status = status;
    const mode = status.providerLive ? "在线" : "离线";
    const companion = status.companionEnabled ? "陪伴已联动" : "基础模式";
    nodes.runtimeStrip.dataset.state = status.providerLive ? "live" : "offline";
    setNodeText(
      nodes.runtimeStrip,
      `${status.model || status.provider}  ${mode}  ${companion}`,
    );
  } catch (error) {
    nodes.runtimeStrip.dataset.state = "error";
    setNodeText(nodes.runtimeStrip, `状态读取失败：${error.message}`);
  }
}

async function sendPrompt(prompt) {
  const clean = text(prompt).trim();
  if (!clean || state.sending) return;
  state.sending = true;
  nodes.send.disabled = true;
  pushMessage("user", clean, "你");
  nodes.prompt.value = "";
  autoResize();

  const started = performance.now();
  pushMessage("assistant", "YunXi 正在回复...", "运行中", "pending");
  const pendingIndex = state.messages.length - 1;

  try {
    const result = await api("/api/chat", {
      method: "POST",
      body: JSON.stringify({ prompt: clean }),
    });
    const elapsed = Math.round(performance.now() - started);
    state.messages[pendingIndex] = {
      role: "assistant",
      text: result.finalResponse || "没有收到最终回复。",
      meta: `${result.provider}/${result.model} · ${result.elapsedMs ?? elapsed} ms · ${result.eventsCount ?? 0} events`,
      ts: Date.now(),
    };
  } catch (error) {
    state.messages[pendingIndex] = {
      role: "assistant",
      text: `回复失败：${error.message}`,
      meta: "错误",
      kind: "error",
      ts: Date.now(),
    };
  } finally {
    state.sending = false;
    nodes.send.disabled = false;
    saveMessages();
    renderMessages();
    loadMemory();
  }
}

function fieldValue(record, key, fallback = "") {
  if (!record) return fallback;
  if (record[key] !== undefined) return record[key];
  const camel = key.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
  return record[camel] ?? fallback;
}

function memoryKind(kind) {
  const labels = {
    correction: "修正",
    preference: "偏好",
    personal_fact: "事实",
    project: "项目",
    relationship: "关系",
    instruction: "指令",
    boundary: "边界",
    routine: "习惯",
  };
  return labels[text(kind).toLowerCase()] || text(kind, "记忆");
}

function memoryStatus(status) {
  const labels = {
    active: "已启用",
    pending: "待确认",
    rejected: "已拒绝",
    archived: "已归档",
  };
  return labels[text(status).toLowerCase()] || text(status, "未知");
}

function scopeText(scope) {
  if (typeof scope === "string") return scope.replaceAll("_", " ");
  if (scope?.workspace) return "workspace";
  if (scope?.global_user || scope?.global) return "global";
  return "memory";
}

function dateText(value) {
  const millis = Number(value);
  if (!Number.isFinite(millis) || millis <= 0) return "时间未知";
  try {
    return new Intl.DateTimeFormat("zh-CN", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(millis));
  } catch {
    return "时间未知";
  }
}

function memoryColor(record, index) {
  const status = text(fieldValue(record, "status", "active")).toLowerCase();
  if (status === "pending") return "#f3c78d";
  if (status === "rejected") return "#f38da4";
  if (status === "archived") return "#a7a7a7";
  const palette = ["#a9d8e9", "#f3c78d", "#9fd7bd", "#d7b3f4"];
  return palette[index % palette.length];
}

function renderMemory(records = []) {
  const field = nodes.memoryField;
  if (!field) return;
  field.innerHTML = "";

  const activeRecords = records.slice(0, 42);
  if (activeRecords.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "还没有可展示的长期记忆";
    field.appendChild(empty);
    return;
  }

  const rect = field.getBoundingClientRect();
  const width = Math.max(320, rect.width || 960);
  const height = Math.max(420, rect.height || 640);
  const count = activeRecords.length;
  const aspect = width / Math.max(1, height);
  const columns = Math.max(1, Math.ceil(Math.sqrt(count * aspect)));
  const rows = Math.max(1, Math.ceil(count / columns));
  const cellWidth = width / columns;
  const cellHeight = height / rows;
  const layoutSeed = hash(`${Math.round(width)}:${Math.round(height)}:${count}`);
  const cells = Array.from({ length: rows * columns }, (_, slot) => ({
    slot,
    column: slot % columns,
    row: Math.floor(slot / columns),
    order: seeded(layoutSeed, slot),
  })).sort((left, right) => left.order - right.order);

  activeRecords.forEach((record, index) => {
    const id = text(fieldValue(record, "id", `memory-${index}`));
    const seed = hash(`${id}:${index}`);
    const cell = cells[index] || cells[cells.length - 1];
    const minCellSize = Math.min(cellWidth, cellHeight);
    const maxSize = clampNumber(minCellSize - 12, 58, 142);
    const baseSize = clampNumber(minCellSize * 0.74, 64, maxSize);
    const size = clampNumber(baseSize + (seeded(seed, 3) - 0.5) * 16, 56, maxSize);
    const slackX = Math.max(0, cellWidth - size - 14);
    const slackY = Math.max(0, cellHeight - size - 14);
    const left = clampNumber(
      cell.column * cellWidth + (cellWidth - size) / 2 + (seeded(seed, 5) - 0.5) * slackX,
      10,
      Math.max(10, width - size - 10),
    );
    const top = clampNumber(
      cell.row * cellHeight + (cellHeight - size) / 2 + (seeded(seed, 7) - 0.5) * slackY,
      10,
      Math.max(10, height - size - 10),
    );
    const floatBudgetX = clampNumber(slackX * 0.24, 3, 16);
    const floatBudgetY = clampNumber(slackY * 0.24, 3, 14);
    const floatX = (seeded(seed, 11) - 0.5) * floatBudgetX;
    const floatY = (seeded(seed, 13) - 0.5) * floatBudgetY;
    const floatR = (seeded(seed, 17) - 0.5) * 6;
    const duration = 7 + seeded(seed, 19) * 8;
    const delay = seeded(seed, 23) * -6;

    const button = document.createElement("button");
    const shortContent = truncate(fieldValue(record, "content"), 18);
    button.type = "button";
    button.className = "memory-orb";
    button.dataset.short = shortContent;
    button.setAttribute("aria-label", `打开记忆：${truncate(fieldValue(record, "content"), 36)}`);
    button.style.setProperty("--orb-size", `${size.toFixed(1)}px`);
    button.style.setProperty("--orb-left", `${left.toFixed(1)}px`);
    button.style.setProperty("--orb-top", `${top.toFixed(1)}px`);
    button.style.setProperty("--float-x", `${floatX.toFixed(1)}px`);
    button.style.setProperty("--float-y", `${floatY.toFixed(1)}px`);
    button.style.setProperty("--float-r", `${floatR.toFixed(2)}deg`);
    button.style.setProperty("--float-duration", `${duration.toFixed(2)}s`);
    button.style.setProperty("--float-delay", `${delay.toFixed(2)}s`);
    button.style.setProperty("--orb-accent", memoryColor(record, index));
    const visual = document.createElement("span");
    visual.className = "memory-orb-visual";
    visual.dataset.short = shortContent;
    button.appendChild(visual);
    button.addEventListener("click", (event) => openMemory(record, event, button));
    field.appendChild(button);
  });
}

async function loadMemory() {
  if (state.memory) {
    renderMemory(state.memory.records || []);
  }
  try {
    const memory = await api("/api/memory");
    state.memory = memory;
    renderMemory(memory.records || []);
  } catch (error) {
    if (!nodes.memoryField) return;
    nodes.memoryField.innerHTML = "";
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = `记忆读取失败：${error.message}`;
    nodes.memoryField.appendChild(empty);
  }
}

function createMetaItem(label, value) {
  const item = document.createElement("div");
  const dt = document.createElement("dt");
  const dd = document.createElement("dd");
  dt.textContent = label;
  dd.textContent = value;
  item.append(dt, dd);
  return item;
}

function renderMemoryDetail(record) {
  const kind = memoryKind(fieldValue(record, "kind", "memory"));
  const status = memoryStatus(fieldValue(record, "status", "active"));
  const scope = scopeText(fieldValue(record, "scope", "memory"));
  const confidence = Number(fieldValue(record, "confidence", 0));
  const confidenceText = Number.isFinite(confidence) ? `${Math.round(confidence * 100)}%` : "未知";
  setNodeText(nodes.memoryDetailKind, kind);
  setNodeText(nodes.memoryDetailTitle, `${status}记忆`);
  setNodeText(nodes.memoryDetailContent, fieldValue(record, "content", "没有内容"));
  if (!nodes.memoryDetailMeta) return;
  nodes.memoryDetailMeta.innerHTML = "";
  nodes.memoryDetailMeta.append(
    createMetaItem("范围", scope),
    createMetaItem("置信度", confidenceText),
    createMetaItem("更新", dateText(fieldValue(record, "updated_at_millis"))),
    createMetaItem("层级", text(fieldValue(record, "layer", "memory"))),
  );
}

function spawnMemoryParticles(x, y, color) {
  const count = prefersReducedMotion() ? 0 : 10;
  for (let index = 0; index < count; index += 1) {
    const particle = document.createElement("span");
    const angle = (Math.PI * 2 * index) / count;
    const distance = 58 + seeded(index + Math.round(x + y), 4) * 104;
    const size = 3 + seeded(index, 9) * 4;
    particle.className = "memory-particle";
    particle.style.setProperty("--particle-x", `${x}px`);
    particle.style.setProperty("--particle-y", `${y}px`);
    particle.style.setProperty("--particle-dx", `${Math.cos(angle) * distance}px`);
    particle.style.setProperty("--particle-dy", `${Math.sin(angle) * distance}px`);
    particle.style.setProperty("--particle-size", `${size}px`);
    particle.style.setProperty("--particle-color", color);
    document.body.appendChild(particle);
    window.setTimeout(() => particle.remove(), 560);
  }
}

function openMemory(record, event, sourceNode) {
  if (!nodes.memoryReveal) return;
  if (state.memoryCloseTimer) {
    window.clearTimeout(state.memoryCloseTimer);
    state.memoryCloseTimer = 0;
  }
  const x = event?.clientX ?? window.innerWidth / 2;
  const y = event?.clientY ?? window.innerHeight / 2;
  const color = sourceNode?.style.getPropertyValue("--orb-accent") || "#f3c78d";
  state.lastMemorySource = sourceNode || null;
  renderMemoryDetail(record);
  nodes.memorySpread?.style.setProperty("--spread-x", `${x}px`);
  nodes.memorySpread?.style.setProperty("--spread-y", `${y}px`);
  nodes.memoryReveal.hidden = false;
  spawnMemoryParticles(x, y, color);
  window.requestAnimationFrame(() => nodes.memoryReveal.classList.add("is-open"));
  if (sourceNode && !prefersReducedMotion()) {
    if (window.gsap) {
      window.gsap.killTweensOf(sourceNode);
      window.gsap.fromTo(
        sourceNode,
        { autoAlpha: 1 },
        {
          autoAlpha: 0.16,
          duration: 0.16,
          ease: "power2.out",
          overwrite: "auto",
          clearProps: "opacity,visibility",
        },
      );
    } else {
      const burst = sourceNode.animate([{ opacity: 1 }, { opacity: 0.16 }], {
        duration: 160,
        easing: "cubic-bezier(0.16, 1, 0.3, 1)",
      });
      burst.finished.then(() => burst.cancel()).catch(() => {});
    }
  }
}

function closeMemory() {
  if (!nodes.memoryReveal) return;
  nodes.memoryReveal.classList.remove("is-open");
  if (state.memoryCloseTimer) window.clearTimeout(state.memoryCloseTimer);
  state.memoryCloseTimer = window.setTimeout(() => {
    nodes.memoryReveal.hidden = true;
    state.lastMemorySource?.focus({ preventScroll: true });
    state.lastMemorySource = null;
    state.memoryCloseTimer = 0;
  }, prefersReducedMotion() ? 1 : 220);
}

function personaLayerLabel(key) {
  const labels = {
    identity: "身份",
    soul: "灵魂",
    values: "价值",
    voice: "语气",
    companion_style: "陪伴",
    work_style: "工作",
    boundaries: "边界",
    addressing: "称呼",
    rules: "规则",
    constraints: "约束",
  };
  return labels[key] || key.replaceAll("_", " ");
}

function buildPersonaLayers(payload) {
  const profile = payload?.profile || {};
  const layers = profile.layers || {};
  const entries = Object.entries(layers).map(([key, value]) => ({
    key,
    label: personaLayerLabel(key),
    title: personaLayerLabel(key),
    content: text(value, "暂无内容"),
  }));

  const rules = profile.companion_rules || profile.companionRules || {};
  if (rules.soul_signature || rules.soulSignature) {
    entries.push({
      key: "rules",
      label: "规则",
      title: "陪伴规则",
      content: [
        rules.soul_signature || rules.soulSignature,
        ...(rules.reply_rules || rules.replyRules || []),
      ]
        .filter(Boolean)
        .join("\n"),
    });
  }

  const constraints = Array.isArray(profile.constraints) ? profile.constraints : [];
  if (constraints.length > 0) {
    entries.push({
      key: "constraints",
      label: "约束",
      title: "硬边界",
      content: constraints.map((item) => item.content || item.id || text(item)).join("\n"),
    });
  }

  if (entries.length === 0) {
    entries.push({
      key: "empty",
      label: "人格",
      title: profile.display_name || profile.displayName || "YunXi Agent",
      content: "还没有读取到人格层内容。",
    });
  }

  return entries;
}

function renderPersona(payload) {
  const deck = nodes.personaDeck;
  if (!deck) return;
  const layers = buildPersonaLayers(payload);
  state.persona = { payload, layers };
  state.activePersonaIndex = Math.min(state.activePersonaIndex, layers.length - 1);
  deck.innerHTML = "";

  layers.forEach((layer, index) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "persona-card";
    card.dataset.index = String(index);
    card.setAttribute("aria-label", `打开人格层：${layer.title}`);
    const body = document.createElement("div");
    const label = document.createElement("small");
    const title = document.createElement("h2");
    const preview = document.createElement("p");
    label.textContent = layer.label;
    title.textContent = layer.title;
    preview.textContent = truncate(layer.content, 118);
    body.append(label, title, preview);

    const footer = document.createElement("div");
    footer.className = "persona-card-footer";
    const indexLabel = document.createElement("span");
    const actionLabel = document.createElement("span");
    indexLabel.textContent = `${index + 1} / ${layers.length}`;
    actionLabel.textContent = "点击查看";
    footer.append(indexLabel, actionLabel);

    card.append(body, footer);
    card.addEventListener("click", () => {
      if (index === state.activePersonaIndex) {
        openPersonaDetail(layer, card);
        return;
      }
      setPersonaIndex(index);
    });
    deck.appendChild(card);
  });

  updatePersonaDeck();
  animatePersonaDeck();
}

async function loadPersona() {
  if (state.persona?.payload) {
    renderPersona(state.persona.payload);
  }
  try {
    const payload = await api("/api/persona");
    renderPersona(payload);
  } catch (error) {
    if (!nodes.personaDeck) return;
    nodes.personaDeck.innerHTML = "";
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = `人格读取失败：${error.message}`;
    nodes.personaDeck.appendChild(empty);
  }
}

function setPersonaIndex(index) {
  const count = state.persona?.layers?.length || 0;
  if (count === 0) return;
  state.activePersonaIndex = (index + count) % count;
  updatePersonaDeck();
}

function updatePersonaDeck() {
  const cards = Array.from(nodes.personaDeck?.querySelectorAll(".persona-card") || []);
  const count = cards.length;
  cards.forEach((card, index) => {
    let offset = index - state.activePersonaIndex;
    if (count > 0) {
      const half = Math.floor(count / 2);
      if (offset > half) offset -= count;
      if (offset < -half) offset += count;
    }
    const lane = clampNumber(offset, -2, 2);
    const absOffset = Math.abs(offset);
    const absLane = Math.abs(lane);
    const visible = absOffset <= 2;
    card.style.setProperty("--offset", String(offset));
    card.style.setProperty("--lane", String(lane));
    card.style.setProperty("--abs-offset", String(absOffset));
    card.style.setProperty("--abs-lane", String(absLane));
    card.classList.toggle("is-active", offset === 0);
    card.classList.toggle("is-persona-visible", visible);
    card.classList.toggle("is-persona-hidden", !visible);
    card.setAttribute("aria-hidden", visible ? "false" : "true");
    card.tabIndex = visible ? 0 : -1;
    card.style.pointerEvents = visible ? "auto" : "none";
    const actionLabel = card.querySelector(".persona-card-footer span:last-child");
    if (actionLabel) {
      actionLabel.textContent = offset === 0 ? "点击查看" : "切换";
    }
  });
}

function setPersonaDetailOrigin(sourceNode) {
  if (!nodes.personaDetail) return;
  const viewportWidth = Math.max(1, window.innerWidth || document.documentElement.clientWidth || 1);
  const viewportHeight = Math.max(1, window.innerHeight || document.documentElement.clientHeight || 1);
  const sourceRect = sourceNode?.getBoundingClientRect?.();
  const hasSource =
    sourceRect &&
    sourceRect.width > 0 &&
    sourceRect.height > 0 &&
    sourceRect.bottom >= 0 &&
    sourceRect.right >= 0 &&
    sourceRect.top <= viewportHeight &&
    sourceRect.left <= viewportWidth;
  const originX = hasSource
    ? clampNumber(sourceRect.left + sourceRect.width / 2, 0, viewportWidth)
    : viewportWidth / 2;
  const originY = hasSource
    ? clampNumber(sourceRect.top + sourceRect.height / 2, 0, viewportHeight)
    : viewportHeight / 2;
  nodes.personaDetail.style.setProperty("--persona-detail-origin-x", `${originX}px`);
  nodes.personaDetail.style.setProperty("--persona-detail-origin-y", `${originY}px`);
}

function personaLayerMeta(layer) {
  const key = layer?.key || "persona";
  const summaries = {
    identity: "它回答时先确认自己是谁，不把本地 Agent 说成云端人格或万能助手。",
    soul: "这是陪伴的底色：真实、持续、克制，不用表演替代理解。",
    values: "它决定什么更重要，先守住诚实、边界和长期一致性。",
    voice: "它控制说话的质感，少一点模板，多一点清晰和自然。",
    companion_style: "它决定怎样陪你往下走，什么时候关心、追问、收住。",
    work_style: "它处理项目时先看现有系统，再做小而明确的改动并验证结果。",
    boundaries: "它标出不能越过的线，安全、隐私和项目硬性规则优先。",
    addressing: "它决定怎么称呼你，优先使用已确认称呼，没确认时保持自然。",
    rules: "它把陪伴细则收束成执行规则，避免回复前后漂移。",
    constraints: "它记录硬约束，任何人格设定都不能覆盖这些底线。",
    empty: "当前没有可展示的人格层内容。",
  };
  return {
    summary: summaries[key] || `这是「${layer?.label || "人格"}」层的具体设定，决定 YunXi 在这个维度上的稳定表现。`,
  };
}

function openPersonaDetail(layer, sourceNode = null) {
  if (!nodes.personaDetail) return;
  if (state.personaCloseTimer) {
    window.clearTimeout(state.personaCloseTimer);
    state.personaCloseTimer = 0;
  }
  state.lastPersonaSource = sourceNode;
  setPersonaDetailOrigin(sourceNode);
  const meta = personaLayerMeta(layer);
  setNodeText(nodes.personaDetailKicker, "人格层");
  setNodeText(nodes.personaDetailTitle, layer.title);
  setNodeText(nodes.personaDetailSummary, meta.summary);
  setNodeText(nodes.personaDetailContent, layer.content);
  nodes.personaDetail.hidden = false;
  window.requestAnimationFrame(() => nodes.personaDetail.classList.add("is-open"));
  const detailParts = nodes.personaDetail.querySelectorAll(
    ".persona-detail-rail > *, .persona-detail-reading > *, .persona-detail-close",
  );
  animateElements(detailParts, { y: 8, duration: 220, stagger: 18 });
  window.setTimeout(() => nodes.personaDetailClose?.focus({ preventScroll: true }), prefersReducedMotion() ? 1 : 120);
}

function closePersonaDetail() {
  if (!nodes.personaDetail) return;
  nodes.personaDetail.classList.remove("is-open");
  if (state.personaCloseTimer) window.clearTimeout(state.personaCloseTimer);
  state.personaCloseTimer = window.setTimeout(() => {
    nodes.personaDetail.hidden = true;
    state.lastPersonaSource?.focus({ preventScroll: true });
    state.lastPersonaSource = null;
    state.personaCloseTimer = 0;
  }, prefersReducedMotion() ? 1 : 220);
}

function animatePersonaDeck() {
  if (state.personaEntrancePlayed) return;
  state.personaEntrancePlayed = true;
  const cards = Array.from(nodes.personaDeck?.querySelectorAll(".persona-card.is-persona-visible") || []);
  const cardParts = cards.flatMap((card) => Array.from(card.children));
  animateElements(cardParts, { y: 6, duration: 220, stagger: 20 });
}

function ensureGsap() {
  if (window.gsap) return Promise.resolve(window.gsap);
  if (state.gsapLoader) return state.gsapLoader;
  if (prefersReducedMotion()) return Promise.resolve(null);

  state.gsapLoader = new Promise((resolve) => {
    const script = document.createElement("script");
    script.src = "https://cdn.jsdelivr.net/npm/gsap@3.12.5/dist/gsap.min.js";
    script.async = true;
    script.dataset.yunxiGsap = "true";
    script.onload = () => resolve(window.gsap || null);
    script.onerror = () => resolve(null);
    document.head.appendChild(script);
  });

  return state.gsapLoader;
}

nodes.navItems.forEach((item) => {
  item.addEventListener("click", () => showView(item.dataset.nav));
});

nodes.prompt?.addEventListener("input", autoResize);
nodes.prompt?.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    nodes.composer?.requestSubmit();
  }
});

nodes.composer?.addEventListener("submit", (event) => {
  event.preventDefault();
  sendPrompt(nodes.prompt?.value || "");
});

nodes.memoryClose?.addEventListener("click", closeMemory);
nodes.memoryReveal?.addEventListener("click", (event) => {
  if (event.target === nodes.memoryReveal || event.target === nodes.memorySpread) {
    closeMemory();
  }
});

nodes.personaPrev?.addEventListener("click", () => setPersonaIndex(state.activePersonaIndex - 1));
nodes.personaNext?.addEventListener("click", () => setPersonaIndex(state.activePersonaIndex + 1));
nodes.personaDetailClose?.addEventListener("click", closePersonaDetail);

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeMemory();
    closePersonaDetail();
  }
  if (document.activeElement instanceof HTMLTextAreaElement) return;
  if (!document.querySelector("#view-persona")?.hidden) {
    if (event.key === "ArrowLeft") setPersonaIndex(state.activePersonaIndex - 1);
    if (event.key === "ArrowRight") setPersonaIndex(state.activePersonaIndex + 1);
  }
});

window.addEventListener("hashchange", () => {
  showView((window.location.hash || "#chat").slice(1));
});

window.addEventListener("resize", () => {
  if (state.resizeFrame) return;
  state.resizeFrame = window.requestAnimationFrame(() => {
    state.resizeFrame = 0;
    if (!document.querySelector("#view-memory")?.hidden && state.memory) {
      renderMemory(state.memory.records || []);
    }
    autoResize();
  });
});

setupDockMotion();
renderMessages();
autoResize();
loadStatus();
showView((window.location.hash || "#chat").slice(1));
