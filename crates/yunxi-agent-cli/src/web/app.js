const STORAGE_KEY = "yunxi-web-redesign-chat";

const HER_EXPRESSION_ASSETS = Object.freeze([
  { src: "/assets/yunxi-her-expression-reserved.jpg", label: "克制" },
  { src: "/assets/yunxi-her-expression-calm.jpg", label: "平静" },
  { src: "/assets/yunxi-her-expression-thoughtful.jpg", label: "思索" },
  { src: "/assets/yunxi-her-expression-wistful.jpg", label: "侧望" },
  { src: "/assets/yunxi-her-expression-smile.jpg", label: "微笑" },
  { src: "/assets/yunxi-her-expression-gentle.jpg", label: "温柔" },
]);

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
  mailboxList: document.querySelector("#mailbox-list"),
  mailboxCount: document.querySelector("#mailbox-count"),
  mailboxRefresh: document.querySelector("#mailbox-refresh"),
  mailboxDetail: document.querySelector("#mailbox-detail"),
  mailboxDetailClose: document.querySelector("#mailbox-detail-close"),
  mailboxDetailTitle: document.querySelector("#mailbox-detail-title"),
  mailboxLetterDate: document.querySelector("#mailbox-letter-date"),
  mailboxLetterBody: document.querySelector("#mailbox-letter-body"),
  mailboxLetterState: document.querySelector("#mailbox-letter-state"),
  mailboxArchive: document.querySelector("#mailbox-archive"),
  herCanvas: document.querySelector("#her-canvas"),
  herDisplayName: document.querySelector("#her-display-name"),
  herIdentity: document.querySelector("#her-identity"),
  herRelationshipLabel: document.querySelector("#her-relationship-label"),
  herRelationshipDescription: document.querySelector("#her-relationship-description"),
  herStagePath: document.querySelector("#her-stage-path"),
  herMemoryCount: document.querySelector("#her-memory-count"),
  herLastCheckIn: document.querySelector("#her-last-check-in"),
  herSoulSignature: document.querySelector("#her-soul-signature"),
  herVoice: document.querySelector("#her-voice"),
  herCompanionStyle: document.querySelector("#her-companion-style"),
  herTraitList: document.querySelector("#her-trait-list"),
  herRuntimeState: document.querySelector("#her-runtime-state"),
  herError: document.querySelector("#her-error"),
  herRetry: document.querySelector("#her-retry"),
  herBackdrop: document.querySelector("#her-backdrop"),
  herBackdropMedia: document.querySelector("#her-backdrop-media"),
  herFlowPrimary: document.querySelector("#her-flow-primary"),
  herFlowSecondary: document.querySelector("#her-flow-secondary"),
  herFlowStreak: document.querySelector("#her-flow-streak"),
  herBackdropBase: document.querySelector("#her-backdrop-base"),
  herBackdropReveal: document.querySelector("#her-backdrop-reveal"),
  herProfileFrame: document.querySelector(".her-profile-frame"),
  herProfileCard: document.querySelector("#her-profile-card"),
  herCardSurface: document.querySelector("#her-card-surface"),
  herCardDepthLayers: Array.from(document.querySelectorAll(".her-card-depth")),
  herCardStage: document.querySelector("#her-card-stage"),
  herDetailDisplayName: document.querySelector("#her-detail-display-name"),
  herArtWindow: document.querySelector("#her-art-window"),
  herArtPlane: document.querySelector("#her-art-plane"),
  herArtImage: document.querySelector("#her-art-image"),
  herArtModes: Array.from(document.querySelectorAll("[data-her-art-mode]")),
};

const state = {
  messages: loadMessages(),
  status: null,
  memory: null,
  persona: null,
  mailbox: null,
  her: null,
  activeMailboxItem: null,
  activePersonaIndex: 0,
  gsapLoader: null,
  viewAnimations: [],
  viewTimeline: null,
  dockTimeline: null,
  lastMessageSignature: null,
  personaEntrancePlayed: false,
  autoResizeFrame: 0,
  resizeFrame: 0,
  memoryCloseTimer: 0,
  personaCloseTimer: 0,
  memoryTimeline: null,
  personaTimeline: null,
  mailboxTimeline: null,
  herTimeline: null,
  herAmbientTimeline: null,
  lastMemorySource: null,
  lastPersonaSource: null,
  lastMailboxSource: null,
  lastMailboxItemId: null,
  mailboxCloseTimer: 0,
  mailboxLoading: false,
  herLoading: false,
  herArtMode: "portrait",
  herExpressionIndex: 0,
  herExpressionsPreloaded: false,
  herArtSwapSequence: 0,
  herRevealTimer: 0,
  sending: false,
};

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function prefersReducedTransparency() {
  return window.matchMedia("(prefers-reduced-transparency: reduce)").matches;
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
  const target = ["chat", "memory", "persona", "her", "mailbox"].includes(name) ? name : "chat";
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
  if (target === "her") loadHer();
  if (target === "mailbox") loadMailbox();
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
  state.viewTimeline?.kill?.();
  state.viewTimeline = null;
  if (name === "her") startHerAmbientMotion();
  else state.herAmbientTimeline?.pause?.();
  if (prefersReducedMotion()) return;
  const view = nodes.views.find((candidate) => candidate.dataset.view === name);
  if (!view || view.hidden) return;
  const selectors = {
    chat: ".chat-hero > *, .chat-panel",
    memory: ".memory-head, .memory-field",
    persona: ".persona-stage",
    her: ".her-scene-index, .her-profile-frame, .her-card-meta, .her-detail-heading, .her-relationship, .her-signals",
    mailbox: ".mailbox-head, .mailbox-list",
  };
  const targets = Array.from(view.querySelectorAll(selectors[name] || ":scope > *"));
  if (window.gsap) {
    state.viewTimeline = window.gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .addLabel("enter")
      .fromTo(view, { autoAlpha: 0 }, { autoAlpha: 1, duration: 0.18, clearProps: "opacity,visibility" }, "enter")
      .fromTo(
        targets,
        { y: 10, autoAlpha: 0 },
        {
          y: 0,
          autoAlpha: 1,
          duration: 0.34,
          stagger: 0.035,
          clearProps: "transform,opacity,visibility",
        },
        "enter+=0.04",
      );
    return;
  }
  state.viewAnimations = animateElements(targets, { y: 6, duration: 240, stagger: 22 });
}

function startHerAmbientMotion() {
  if (!nodes.herBackdropMedia || prefersReducedMotion() || prefersReducedTransparency()) return;
  if (state.herAmbientTimeline) {
    state.herAmbientTimeline.resume();
    return;
  }
  ensureGsap().then((gsap) => {
    if (state.herAmbientTimeline || document.querySelector("#view-her")?.hidden) return;
    if (gsap) {
      state.herAmbientTimeline = gsap
        .timeline({ repeat: -1, yoyo: true, defaults: { ease: "sine.inOut" } })
        .fromTo(
          nodes.herBackdropMedia,
          { xPercent: -0.2, scale: 1.012 },
          { xPercent: 0.35, scale: 1.025, duration: 14 },
          0,
        )
        .fromTo(
          nodes.herFlowPrimary,
          { xPercent: -28, yPercent: -4, rotation: -8, opacity: 0.08 },
          { xPercent: 62, yPercent: 7, rotation: -5.5, opacity: 0.34, duration: 17 },
          0,
        )
        .fromTo(
          nodes.herFlowSecondary,
          { xPercent: 34, yPercent: 5, rotation: 6, opacity: 0.06 },
          { xPercent: -48, yPercent: -7, rotation: 3.5, opacity: 0.24, duration: 21 },
          0,
        )
        .fromTo(
          nodes.herFlowStreak,
          { xPercent: -18, rotation: -5, opacity: 0.02 },
          { xPercent: 118, rotation: -3, opacity: 0.22, duration: 13 },
          2,
        );
      return;
    }
    const animations = [nodes.herBackdropMedia.animate(
      [
        { transform: "translate3d(-0.2%, 0, 0) scale(1.012)" },
        { transform: "translate3d(0.35%, 0, 0) scale(1.025)" },
      ],
      { duration: 28000, direction: "alternate", iterations: Infinity, easing: "ease-in-out" },
    )];
    animations.push(
      nodes.herFlowPrimary.animate(
        [
          { transform: "translate3d(-28%, -4%, 0) rotate(-8deg)", opacity: 0.08 },
          { transform: "translate3d(62%, 7%, 0) rotate(-5.5deg)", opacity: 0.34 },
        ],
        { duration: 34000, direction: "alternate", iterations: Infinity, easing: "ease-in-out" },
      ),
      nodes.herFlowSecondary.animate(
        [
          { transform: "translate3d(34%, 5%, 0) rotate(6deg)", opacity: 0.06 },
          { transform: "translate3d(-48%, -7%, 0) rotate(3.5deg)", opacity: 0.24 },
        ],
        { duration: 42000, direction: "alternate", iterations: Infinity, easing: "ease-in-out" },
      ),
      nodes.herFlowStreak.animate(
        [
          { transform: "translate3d(-18%, 0, 0) rotate(-5deg)", opacity: 0.02 },
          { transform: "translate3d(118%, 0, 0) rotate(-3deg)", opacity: 0.22 },
        ],
        { duration: 26000, direction: "alternate", iterations: Infinity, easing: "ease-in-out" },
      ),
    );
    state.herAmbientTimeline = {
      pause: () => animations.forEach((animation) => animation.pause()),
      resume: () => animations.forEach((animation) => animation.play()),
      kill: () => animations.forEach((animation) => animation.cancel()),
    };
  });
}

function animateDockSelection(item) {
  const icon = item?.querySelector(".dock-icon");
  if (!icon || prefersReducedMotion()) return;
  if (window.gsap) {
    state.dockTimeline?.kill?.();
    state.dockTimeline = window.gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .to(icon, { y: 1, scale: 0.84, duration: 0.08 })
      .to(icon, { y: -1, scale: 1.12, duration: 0.15 })
      .to(icon, { y: 0, scale: 1, duration: 0.18, clearProps: "transform" });
    return;
  }
  const feedback = icon.animate(
    [
      { transform: "translate3d(0, 1px, 0) scale(0.84)" },
      { transform: "translate3d(0, -1px, 0) scale(1.12)" },
      { transform: "translate3d(0, 0, 0) scale(1)" },
    ],
    { duration: 360, easing: "cubic-bezier(0.16, 1, 0.3, 1)" },
  );
  feedback.finished.then(() => feedback.cancel()).catch(() => {});
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
      const strength = window.gsap?.utils
        ? window.gsap.utils.clamp(0, 1, window.gsap.utils.mapRange(106, 0, 0, 1, distance))
        : clampNumber(1 - distance / 106, 0, 1);
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
    nodes.dock.classList.remove("is-tooltip-suppressed");
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
  items.forEach((item) => {
    item.addEventListener("click", () => {
      nodes.dock.classList.add("is-tooltip-suppressed");
      animateDockSelection(item);
    });
  });
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
  const latestMessage = state.messages.at(-1);
  const latestSignature = latestMessage
    ? `${latestMessage.role}:${latestMessage.kind || ""}:${latestMessage.ts || 0}:${latestMessage.text || ""}`
    : "";
  const shouldAnimateLatest = latestSignature !== state.lastMessageSignature;
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
  state.lastMessageSignature = latestSignature;
  if (shouldAnimateLatest && latestMessage) {
    const latestArticle = nodes.chatLog.lastElementChild;
    window.requestAnimationFrame(() => animateMessageArrival(latestArticle));
  }
}

function animateMessageArrival(article) {
  if (!article || prefersReducedMotion()) return;
  const bubble = article.querySelector(".message-bubble");
  if (window.gsap) {
    window.gsap.killTweensOf([article, bubble]);
    window.gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .fromTo(article, { y: 8, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.28, clearProps: "transform,opacity,visibility" })
      .fromTo(bubble, { scale: 0.988 }, { scale: 1, duration: 0.3, clearProps: "transform" }, "<");
    return;
  }
  const arrival = article.animate(
    [
      { opacity: 0, transform: "translate3d(0, 8px, 0)" },
      { opacity: 1, transform: "translate3d(0, 0, 0)" },
    ],
    { duration: 280, easing: "cubic-bezier(0.16, 1, 0.3, 1)" },
  );
  arrival.finished.then(() => arrival.cancel()).catch(() => {});
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
    loadHer(true);
    loadMailbox(true);
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

function memoryColor(record) {
  const status = text(fieldValue(record, "status", "active")).toLowerCase();
  if (status === "pending") return "#c5a16f";
  if (status === "rejected") return "#bd858c";
  if (status === "archived") return "#858d89";
  return "#9fc5d6";
}

function memoryOrbLabel(record) {
  const source = text(fieldValue(record, "content"))
    .replace(/\s+/g, " ")
    .trim();
  const cleaned = source
    .replace(/^(?:用户纠正\s*\/\s*限制|目标候选|事实候选|偏好候选|项目规则候选|长期记忆|记忆)\s*[：:]\s*/u, "")
    .replace(/^请测试长期记忆\s*[：:]\s*/u, "")
    .trim();
  const firstSentence = cleaned.split(/[。！？!?；;\n]/u)[0]?.trim() || cleaned;
  return truncate(firstSentence || source, 13);
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
    const breathDuration = 5.6 + seeded(seed, 29) * 3.2;
    const breathDelay = seeded(seed, 31) * -breathDuration;
    const breathScale = 1.055 + seeded(seed, 37) * 0.04;
    const breathOpacity = 0.34 + seeded(seed, 41) * 0.16;

    const button = document.createElement("button");
    const shortContent = memoryOrbLabel(record);
    const status = text(fieldValue(record, "status", "active")).toLowerCase();
    button.type = "button";
    button.className = "memory-orb";
    button.classList.toggle("is-compact", size < 78);
    button.dataset.short = shortContent;
    button.dataset.status = status;
    button.setAttribute("aria-label", `打开记忆：${truncate(fieldValue(record, "content"), 36)}`);
    button.style.setProperty("--orb-size", `${size.toFixed(1)}px`);
    button.style.setProperty("--orb-left", `${left.toFixed(1)}px`);
    button.style.setProperty("--orb-top", `${top.toFixed(1)}px`);
    button.style.setProperty("--float-x", `${floatX.toFixed(1)}px`);
    button.style.setProperty("--float-y", `${floatY.toFixed(1)}px`);
    button.style.setProperty("--float-r", `${floatR.toFixed(2)}deg`);
    button.style.setProperty("--float-duration", `${duration.toFixed(2)}s`);
    button.style.setProperty("--float-delay", `${delay.toFixed(2)}s`);
    button.style.setProperty("--breath-duration", `${breathDuration.toFixed(2)}s`);
    button.style.setProperty("--breath-delay", `${breathDelay.toFixed(2)}s`);
    button.style.setProperty("--breath-scale", breathScale.toFixed(3));
    button.style.setProperty("--breath-opacity", breathOpacity.toFixed(3));
    button.style.setProperty("--orb-accent", memoryColor(record));
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
  const color = sourceNode?.style.getPropertyValue("--orb-accent") || "#9fc5d6";
  state.lastMemorySource = sourceNode || null;
  renderMemoryDetail(record);
  nodes.memorySpread?.style.setProperty("--spread-x", `${x}px`);
  nodes.memorySpread?.style.setProperty("--spread-y", `${y}px`);
  nodes.memoryReveal.hidden = false;
  spawnMemoryParticles(x, y, color);
  window.requestAnimationFrame(() => {
    nodes.memoryReveal.classList.add("is-open");
    animateMemoryReveal();
  });
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
  state.memoryTimeline?.kill?.();
  state.memoryTimeline = null;
  nodes.memoryReveal.classList.remove("is-open");
  if (state.memoryCloseTimer) window.clearTimeout(state.memoryCloseTimer);
  state.memoryCloseTimer = window.setTimeout(() => {
    nodes.memoryReveal.hidden = true;
    state.lastMemorySource?.focus({ preventScroll: true });
    state.lastMemorySource = null;
    state.memoryCloseTimer = 0;
  }, prefersReducedMotion() ? 1 : 220);
}

function mailboxStateLabel(value) {
  const labels = {
    unread: "未读",
    read: "已读",
    archived: "已归档",
  };
  return labels[text(value).toLowerCase()] || "信件";
}

function mailboxDate(value) {
  const millis = Number(value);
  if (!Number.isFinite(millis) || millis <= 0) return "时间未知";
  try {
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(millis));
  } catch {
    return "时间未知";
  }
}

function renderMailboxLoading() {
  if (!nodes.mailboxList) return;
  nodes.mailboxList.setAttribute("aria-busy", "true");
  nodes.mailboxList.innerHTML = "";
  const loading = document.createElement("div");
  loading.className = "mailbox-loading";
  loading.setAttribute("aria-label", "正在读取信件");
  for (let index = 0; index < 3; index += 1) {
    loading.appendChild(document.createElement("span"));
  }
  nodes.mailboxList.appendChild(loading);
  setNodeText(nodes.mailboxCount, "正在读取信件");
}

function renderMailboxError(error) {
  if (!nodes.mailboxList) return;
  nodes.mailboxList.setAttribute("aria-busy", "false");
  nodes.mailboxList.innerHTML = "";
  const empty = document.createElement("div");
  empty.className = "mailbox-empty";
  const title = document.createElement("h2");
  const detail = document.createElement("p");
  const retry = document.createElement("button");
  title.textContent = "信箱暂时无法打开";
  detail.textContent = text(error?.message, "读取失败");
  retry.type = "button";
  retry.className = "mailbox-retry";
  retry.textContent = "重试";
  retry.addEventListener("click", () => loadMailbox(true));
  empty.append(title, detail, retry);
  nodes.mailboxList.appendChild(empty);
  setNodeText(nodes.mailboxCount, "读取失败");
}

function renderMailbox(payload) {
  if (!nodes.mailboxList) return;
  const items = Array.isArray(payload?.items) ? payload.items : [];
  const unreadCount = Number(payload?.unreadCount) || 0;
  nodes.mailboxList.setAttribute("aria-busy", "false");
  nodes.mailboxList.innerHTML = "";
  setNodeText(nodes.mailboxCount, unreadCount > 0 ? `${unreadCount} 封未读` : "没有未读信件");

  if (items.length === 0) {
    const empty = document.createElement("div");
    empty.className = "mailbox-empty";
    const title = document.createElement("h2");
    const detail = document.createElement("p");
    title.textContent = "还没有信件";
    detail.textContent = "新的信会安静地留在这里。";
    empty.append(title, detail);
    nodes.mailboxList.appendChild(empty);
    return;
  }

  for (const item of items) {
    const button = document.createElement("button");
    const meta = document.createElement("span");
    const stateLabel = document.createElement("span");
    const date = document.createElement("time");
    const subject = document.createElement("h2");
    const preview = document.createElement("p");
    const itemState = text(item.state, "read").toLowerCase();
    button.type = "button";
    button.className = `mailbox-item is-${itemState}`;
    button.dataset.mailboxItem = text(item.itemId);
    button.setAttribute("aria-label", `打开信件：${text(item.subject, "一封信")}`);
    stateLabel.className = "mailbox-item-state";
    stateLabel.textContent = mailboxStateLabel(itemState);
    date.dateTime = new Date(Number(item.availableAtMillis) || 0).toISOString();
    date.textContent = mailboxDate(item.availableAtMillis);
    meta.className = "mailbox-item-meta";
    meta.append(stateLabel, date);
    subject.textContent = text(item.subject, "一封来自 YunXi 的信");
    preview.textContent = text(item.preview, "打开阅读正文");
    button.append(meta, subject, preview);
    button.addEventListener("click", () => openMailbox(item, button));
    nodes.mailboxList.appendChild(button);
  }

  animateElements(nodes.mailboxList.querySelectorAll(".mailbox-item"), {
    y: 7,
    duration: 300,
    stagger: 28,
  });
}

async function loadMailbox(force = false) {
  if (state.mailboxLoading) return;
  if (state.mailbox && !force) renderMailbox(state.mailbox);
  if (!state.mailbox || force) renderMailboxLoading();
  state.mailboxLoading = true;
  nodes.mailboxRefresh?.setAttribute("aria-busy", "true");
  nodes.mailboxRefresh?.setAttribute("disabled", "");
  try {
    const payload = await api("/api/mailbox");
    state.mailbox = payload;
    renderMailbox(payload);
  } catch (error) {
    renderMailboxError(error);
  } finally {
    state.mailboxLoading = false;
    nodes.mailboxRefresh?.removeAttribute("aria-busy");
    nodes.mailboxRefresh?.removeAttribute("disabled");
  }
}

function updateMailboxCache(item) {
  if (!state.mailbox || !item) return;
  const index = state.mailbox.items?.findIndex((candidate) => candidate.itemId === item.itemId) ?? -1;
  if (index >= 0) state.mailbox.items[index] = item;
  state.mailbox.unreadCount = (state.mailbox.items || []).filter(
    (candidate) => candidate.state === "unread",
  ).length;
}

function renderMailboxDetail(detail) {
  const item = detail?.item || state.activeMailboxItem || {};
  state.activeMailboxItem = item;
  setNodeText(nodes.mailboxDetailTitle, detail?.subject || item.subject || "一封信");
  setNodeText(nodes.mailboxLetterDate, mailboxDate(item.availableAtMillis));
  setNodeText(nodes.mailboxLetterBody, detail?.body || "正文暂时无法读取");
  setNodeText(nodes.mailboxLetterState, mailboxStateLabel(item.state));
  if (nodes.mailboxArchive) {
    const archived = item.state === "archived";
    nodes.mailboxArchive.disabled = archived;
    nodes.mailboxArchive.querySelector("span").textContent = archived ? "已归档" : "归档";
  }
}

async function openMailbox(item, sourceNode) {
  if (!nodes.mailboxDetail || !item?.itemId) return;
  if (state.mailboxCloseTimer) {
    window.clearTimeout(state.mailboxCloseTimer);
    state.mailboxCloseTimer = 0;
  }
  state.lastMailboxSource = sourceNode || null;
  state.lastMailboxItemId = item.itemId;
  state.activeMailboxItem = item;
  renderMailboxDetail({ item, subject: item.subject, body: "正在解密信件..." });
  nodes.mailboxDetail.hidden = false;
  window.requestAnimationFrame(() => {
    nodes.mailboxDetail.classList.add("is-open");
    animateMailboxDetail();
  });
  window.setTimeout(
    () => nodes.mailboxDetailClose?.focus({ preventScroll: true }),
    prefersReducedMotion() ? 1 : 120,
  );

  try {
    let detail = await api(`/api/mailbox/${encodeURIComponent(item.itemId)}`);
    if (detail.item?.state === "unread") {
      detail = await api(`/api/mailbox/${encodeURIComponent(item.itemId)}/state`, {
        method: "POST",
        body: JSON.stringify({ state: "read" }),
      });
    }
    updateMailboxCache(detail.item);
    renderMailbox(state.mailbox);
    renderMailboxDetail(detail);
  } catch (error) {
    setNodeText(nodes.mailboxLetterBody, `信件读取失败：${error.message}`);
  }
}

async function archiveActiveMailbox() {
  const item = state.activeMailboxItem;
  if (!item?.itemId || item.state === "archived") return;
  nodes.mailboxArchive.disabled = true;
  try {
    const detail = await api(`/api/mailbox/${encodeURIComponent(item.itemId)}/state`, {
      method: "POST",
      body: JSON.stringify({ state: "archived" }),
    });
    updateMailboxCache(detail.item);
    renderMailbox(state.mailbox);
    renderMailboxDetail(detail);
  } catch (error) {
    setNodeText(nodes.mailboxLetterState, `归档失败：${error.message}`);
    nodes.mailboxArchive.disabled = false;
  }
}

function closeMailboxDetail() {
  if (!nodes.mailboxDetail) return;
  state.mailboxTimeline?.kill?.();
  state.mailboxTimeline = null;
  nodes.mailboxDetail.classList.remove("is-open");
  if (state.mailboxCloseTimer) window.clearTimeout(state.mailboxCloseTimer);
  state.mailboxCloseTimer = window.setTimeout(() => {
    nodes.mailboxDetail.hidden = true;
    const updatedSource = Array.from(nodes.mailboxList?.querySelectorAll(".mailbox-item") || []).find(
      (candidate) => candidate.dataset.mailboxItem === state.lastMailboxItemId,
    );
    const focusTarget = state.lastMailboxSource?.isConnected ? state.lastMailboxSource : updatedSource;
    focusTarget?.focus({ preventScroll: true });
    state.lastMailboxSource = null;
    state.lastMailboxItemId = null;
    state.activeMailboxItem = null;
    state.mailboxCloseTimer = 0;
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

function buildAuthoritativeSoulSections(source) {
  const content = typeof source === "string" ? source : "";
  const matches = Array.from(content.matchAll(/^\*\*(\d+)\.\s*(.+?)\*\*\s*$/gm));
  if (matches.length === 0) {
    return [
      {
        key: "soul_section_1",
        label: "原文",
        title: "soul.txt",
        content,
      },
    ];
  }
  return matches.map((match, index) => {
    const start = match.index || 0;
    const end = matches[index + 1]?.index ?? content.length;
    return {
      key: `soul_section_${match[1]}`,
      label: `原文 ${match[1]}`,
      title: match[2],
      content: content.slice(start, end),
    };
  });
}

function buildPersonaLayers(payload) {
  const profile = payload?.profile || {};
  const layers = profile.layers || {};
  const authoritativeSoul = profile.authoritative_soul || profile.authoritativeSoul;
  if (authoritativeSoul) {
    return buildAuthoritativeSoulSections(layers.soul);
  }
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

    card.append(body);
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
    const lane = clampNumber(offset, -1, 1);
    const absOffset = Math.abs(offset);
    const absLane = Math.abs(lane);
    const visible = absOffset <= 1;
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
  });
}

function formatHerCheckIn(value) {
  const millis = Number(value);
  if (!Number.isFinite(millis) || millis <= 0) return "尚未形成记录";
  try {
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "long",
      day: "numeric",
    }).format(new Date(millis));
  } catch {
    return "时间未知";
  }
}

function renderHerTraits(traits) {
  if (!nodes.herTraitList) return;
  nodes.herTraitList.innerHTML = "";
  for (const trait of Array.isArray(traits) ? traits : []) {
    const item = document.createElement("div");
    const label = document.createElement("span");
    const level = document.createElement("strong");
    item.className = "her-trait";
    item.dataset.trait = text(trait.id);
    item.style.setProperty("--trait-progress", `${clampNumber(Number(trait.score) || 0, 0, 100)}%`);
    label.textContent = text(trait.label, "性格");
    level.textContent = text(trait.level, "平衡");
    item.append(label, level);
    nodes.herTraitList.appendChild(item);
  }
}

function renderHerRuntime(payload) {
  if (!nodes.herRuntimeState) return;
  nodes.herRuntimeState.innerHTML = "";
  const statuses = [
    ["人格", Boolean(payload?.personaEnabled)],
    ["记忆", Boolean(payload?.memoryEnabled)],
    ["陪伴", Boolean(payload?.companionEnabled)],
  ];
  for (const [label, enabled] of statuses) {
    const item = document.createElement("span");
    item.className = enabled ? "is-online" : "is-offline";
    item.textContent = `${label}${enabled ? "在线" : "未启用"}`;
    nodes.herRuntimeState.appendChild(item);
  }
}

function renderHer(payload) {
  state.her = payload;
  setNodeText(nodes.herDisplayName, text(payload?.displayName, "YunXi"));
  setNodeText(nodes.herDetailDisplayName, text(payload?.displayName, "YunXi"));
  setNodeText(nodes.herIdentity, text(payload?.identity, "她的身份档案暂时为空。"));
  setNodeText(nodes.herRelationshipLabel, text(payload?.relationshipLabel, "初识"));
  setNodeText(nodes.herCardStage, text(payload?.relationshipLabel, "初识"));
  setNodeText(nodes.herRelationshipDescription, text(payload?.relationshipDescription));
  setNodeText(
    nodes.herMemoryCount,
    `${Number(payload?.meaningfulMemoryCount) || 0} 条关系记忆 · ${Number(payload?.activeMemoryCount) || 0} 条可用`,
  );
  setNodeText(nodes.herLastCheckIn, formatHerCheckIn(payload?.lastMeaningfulCheckInMillis));
  setNodeText(nodes.herSoulSignature, text(payload?.soulSignature, "尚未设置"));
  setNodeText(nodes.herVoice, text(payload?.voice, "尚未设置"));
  setNodeText(nodes.herCompanionStyle, text(payload?.companionStyle, "尚未设置"));
  const stages = ["new", "familiar", "established"];
  const currentIndex = Math.max(0, stages.indexOf(text(payload?.relationshipStage, "new")));
  nodes.herStagePath?.querySelectorAll("[data-her-stage]").forEach((item, index) => {
    item.classList.toggle("is-current", index === currentIndex);
    item.classList.toggle("is-reached", index <= currentIndex);
    if (index === currentIndex) item.setAttribute("aria-current", "step");
    else item.removeAttribute("aria-current");
  });
  renderHerTraits(payload?.traits);
  renderHerRuntime(payload);
  nodes.herCanvas?.classList.remove("is-loading", "has-error");
  nodes.herCanvas?.setAttribute("aria-busy", "false");
  if (nodes.herError) nodes.herError.hidden = true;
}

function renderHerError() {
  nodes.herCanvas?.classList.remove("is-loading");
  nodes.herCanvas?.classList.add("has-error");
  nodes.herCanvas?.setAttribute("aria-busy", "false");
  if (nodes.herError) nodes.herError.hidden = false;
}

async function loadHer(force = false) {
  if (state.herLoading) return;
  if (state.her && !force) renderHer(state.her);
  if (!state.her || force) {
    nodes.herCanvas?.classList.add("is-loading");
    nodes.herCanvas?.setAttribute("aria-busy", "true");
  }
  state.herLoading = true;
  try {
    renderHer(await api("/api/her"));
  } catch {
    renderHerError();
  } finally {
    state.herLoading = false;
  }
}

function preloadHerExpressions() {
  if (state.herExpressionsPreloaded) return;
  state.herExpressionsPreloaded = true;
  HER_EXPRESSION_ASSETS.forEach(({ src }) => {
    const image = new Image();
    image.decoding = "async";
    image.src = src;
  });
}

function animateHerArtSwap(sequence) {
  if (sequence !== state.herArtSwapSequence || prefersReducedMotion() || !nodes.herArtImage) return;
  ensureGsap().then((gsap) => {
    if (!gsap || !nodes.herArtImage || sequence !== state.herArtSwapSequence) return;
    state.herTimeline?.kill?.();
    state.herTimeline = gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .fromTo(
        nodes.herArtImage,
        { autoAlpha: 0.46 },
        { autoAlpha: 1, duration: 0.36, clearProps: "opacity,visibility" },
      );
  });
}

function setHerArtMode(mode, { advanceExpression = false } = {}) {
  const nextMode = ["portrait", "expressions"].includes(mode) ? mode : "portrait";
  if (nextMode === "expressions") {
    preloadHerExpressions();
    if (advanceExpression && state.herArtMode === "expressions") {
      state.herExpressionIndex = (state.herExpressionIndex + 1) % HER_EXPRESSION_ASSETS.length;
    }
  }
  state.herArtMode = nextMode;
  const expression = HER_EXPRESSION_ASSETS[state.herExpressionIndex];
  if (nodes.herArtWindow) {
    nodes.herArtWindow.dataset.artMode = nextMode;
    nodes.herArtWindow.dataset.expressionIndex = String(state.herExpressionIndex);
  }
  const sequence = ++state.herArtSwapSequence;
  if (nodes.herArtImage) {
    const profileSource = nodes.herArtImage.dataset.profileSrc || "/assets/yunxi-her-profile-card.jpg";
    const nextSource = nextMode === "portrait" ? profileSource : expression.src;
    let revealed = false;
    const reveal = () => {
      if (revealed) return;
      revealed = true;
      animateHerArtSwap(sequence);
    };
    if (nodes.herArtImage.getAttribute("src") !== nextSource) {
      nodes.herArtImage.addEventListener("load", reveal, { once: true });
      nodes.herArtImage.setAttribute("src", nextSource);
      if (nodes.herArtImage.complete && nodes.herArtImage.naturalWidth > 0) queueMicrotask(reveal);
    } else {
      reveal();
    }
    nodes.herArtImage.alt = nextMode === "portrait" ? "YunXi 人物形象" : `YunXi ${expression.label}神情`;
  }
  nodes.herArtModes.forEach((button) => {
    const active = button.dataset.herArtMode === nextMode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
    if (button.dataset.herArtMode === "expressions") {
      const expressionPosition = `${state.herExpressionIndex + 1}/${HER_EXPRESSION_ASSETS.length}`;
      const label = active
        ? `${expression.label}，表情 ${expressionPosition}，再次点击切换`
        : "查看表情图";
      button.setAttribute("aria-label", label);
      button.title = active ? `${expression.label} · ${expressionPosition}` : "表情";
    }
  });
}

function setupHerReveal() {
  if (!nodes.herCanvas || !nodes.herBackdrop) return;
  const supportsFinePointer = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  if (!supportsFinePointer || prefersReducedMotion()) return;

  let xTo = null;
  let yTo = null;
  const spotlight = { x: 0, y: 0 };
  const syncSpotlight = () => {
    nodes.herBackdrop?.style.setProperty("--spot-x", `${spotlight.x.toFixed(1)}px`);
    nodes.herBackdrop?.style.setProperty("--spot-y", `${spotlight.y.toFixed(1)}px`);
  };
  ensureGsap().then((gsap) => {
    if (!gsap || !nodes.herBackdrop) return;
    xTo = gsap.quickTo(spotlight, "x", { duration: 0.42, ease: "power3.out", onUpdate: syncSpotlight });
    yTo = gsap.quickTo(spotlight, "y", { duration: 0.42, ease: "power3.out", onUpdate: syncSpotlight });
  });

  const holdReveal = () => {
    window.clearTimeout(state.herRevealTimer);
    state.herRevealTimer = window.setTimeout(() => {
      nodes.herBackdrop?.classList.remove("is-revealing");
      nodes.herBackdrop?.classList.add("is-fading");
      window.setTimeout(() => nodes.herBackdrop?.classList.remove("is-fading"), 1600);
    }, 900);
  };

  nodes.herCanvas.addEventListener("pointermove", (event) => {
    const rect = nodes.herCanvas.getBoundingClientRect();
    const x = clampNumber(event.clientX - rect.left, 0, rect.width);
    const y = clampNumber(event.clientY - rect.top, 0, rect.height);
    nodes.herBackdrop.classList.remove("is-fading");
    nodes.herBackdrop.classList.add("is-revealing");
    if (xTo && yTo) {
      xTo(x);
      yTo(y);
    } else {
      spotlight.x = x;
      spotlight.y = y;
      syncSpotlight();
    }
    holdReveal();
  });
  nodes.herCanvas.addEventListener("pointerleave", holdReveal);
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
  if (key.startsWith("soul_section_")) {
    return {
      summary: "来自 soul.txt 的原始章节，内容未改写。",
    };
  }
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
  setNodeText(
    nodes.personaDetailKicker,
    layer.key?.startsWith("soul_section_") ? "soul.txt" : "人格层",
  );
  setNodeText(nodes.personaDetailTitle, layer.title);
  setNodeText(nodes.personaDetailSummary, meta.summary);
  setNodeText(nodes.personaDetailContent, layer.content);
  nodes.personaDetail.hidden = false;
  window.requestAnimationFrame(() => {
    nodes.personaDetail.classList.add("is-open");
    animatePersonaDetail();
  });
  window.setTimeout(() => nodes.personaDetailClose?.focus({ preventScroll: true }), prefersReducedMotion() ? 1 : 120);
}

function closePersonaDetail() {
  if (!nodes.personaDetail) return;
  state.personaTimeline?.kill?.();
  state.personaTimeline = null;
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

function animateMemoryReveal() {
  if (prefersReducedMotion() || !nodes.memoryReveal) return;
  ensureGsap().then((gsap) => {
    if (!gsap || nodes.memoryReveal.hidden || !nodes.memoryReveal.classList.contains("is-open")) return;
    const card = nodes.memoryReveal.querySelector(".memory-detail-card");
    const parts = card?.querySelectorAll(".detail-kicker, h2, #memory-detail-content, .detail-grid > *") || [];
    state.memoryTimeline?.kill?.();
    state.memoryTimeline = gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .addLabel("reveal")
      .fromTo(nodes.memorySpread, { scale: 0.9, autoAlpha: 0 }, { scale: 1, autoAlpha: 1, duration: 0.34 }, "reveal")
      .fromTo(card, { y: 16, scale: 0.985, autoAlpha: 0 }, { y: 0, scale: 1, autoAlpha: 1, duration: 0.4 }, "reveal+=0.08")
      .fromTo(parts, { y: 7, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.24, stagger: 0.025 }, "reveal+=0.18");
  });
}

function animatePersonaDetail() {
  if (prefersReducedMotion() || !nodes.personaDetail) return;
  ensureGsap().then((gsap) => {
    if (!gsap || nodes.personaDetail.hidden || !nodes.personaDetail.classList.contains("is-open")) return;
    const layout = nodes.personaDetail.querySelector(".persona-detail-layout");
    const railParts = nodes.personaDetail.querySelectorAll(".persona-detail-rail > *");
    const readingParts = nodes.personaDetail.querySelectorAll(".persona-detail-reading > *");
    state.personaTimeline?.kill?.();
    state.personaTimeline = gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .addLabel("open")
      .fromTo(layout, { y: 18, scale: 0.99, autoAlpha: 0 }, { y: 0, scale: 1, autoAlpha: 1, duration: 0.38 }, "open")
      .fromTo(railParts, { y: 8, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.26, stagger: 0.035 }, "open+=0.12")
      .fromTo(readingParts, { y: 10, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.3, stagger: 0.04 }, "open+=0.18");
  });
}

function animateMailboxDetail() {
  if (prefersReducedMotion() || !nodes.mailboxDetail) return;
  ensureGsap().then((gsap) => {
    if (!gsap || nodes.mailboxDetail.hidden || !nodes.mailboxDetail.classList.contains("is-open")) return;
    const letter = nodes.mailboxDetail.querySelector(".mailbox-letter");
    const parts = letter?.querySelectorAll(
      ".mailbox-letter-kicker, h2, .mailbox-letter-date, .mailbox-letter-body, .mailbox-letter-actions",
    ) || [];
    state.mailboxTimeline?.kill?.();
    state.mailboxTimeline = gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .addLabel("open")
      .fromTo(letter, { y: 18, scale: 0.985, autoAlpha: 0 }, { y: 0, scale: 1, autoAlpha: 1, duration: 0.38 }, "open")
      .fromTo(parts, { y: 7, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.26, stagger: 0.035 }, "open+=0.12");
  });
}

function setupPointerMaterials() {
  const supportsFinePointer = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  if (!supportsFinePointer || prefersReducedMotion()) return;

  const personaControls = new WeakMap();
  let herControls = null;
  const resetSurface = (surface, onReset) => {
    if (!surface) return;
    surface.classList.remove("is-pointer-active");
    onReset?.(surface);
  };

  const bindSurface = (root, selector, onUpdate, onReset) => {
    if (!root) return;
    let activeSurface = null;
    root.addEventListener("pointermove", (event) => {
      const surface = selector ? event.target.closest(selector) : root;
      if (!surface || !root.contains(surface)) {
        resetSurface(activeSurface, onReset);
        activeSurface = null;
        return;
      }
      if (surface !== activeSurface) {
        resetSurface(activeSurface, onReset);
        activeSurface = surface;
        activeSurface.classList.add("is-pointer-active");
      }
      const rect = surface.getBoundingClientRect();
      const x = clampNumber((event.clientX - rect.left) / Math.max(1, rect.width), 0, 1);
      const y = clampNumber((event.clientY - rect.top) / Math.max(1, rect.height), 0, 1);
      surface.style.setProperty("--pointer-x", `${(x * 100).toFixed(2)}%`);
      surface.style.setProperty("--pointer-y", `${(y * 100).toFixed(2)}%`);
      onUpdate?.(surface, x, y);
    });
    root.addEventListener("pointerleave", () => {
      resetSurface(activeSurface, onReset);
      activeSurface = null;
    });
  };

  const updatePersonaTilt = (card, x, y) => {
    if (!card.classList.contains("is-active")) return;
    const content = card.firstElementChild;
    if (!content) return;
    if (window.gsap) {
      let controls = personaControls.get(content);
      if (!controls) {
        window.gsap.set(content, { transformPerspective: 900, transformOrigin: "50% 50%" });
        controls = {
          rotationX: window.gsap.quickTo(content, "rotationX", { duration: 0.28, ease: "power3.out" }),
          rotationY: window.gsap.quickTo(content, "rotationY", { duration: 0.28, ease: "power3.out" }),
          y: window.gsap.quickTo(content, "y", { duration: 0.28, ease: "power3.out" }),
        };
        personaControls.set(content, controls);
      }
      controls.rotationX((0.5 - y) * 3.2);
      controls.rotationY((x - 0.5) * 4.2);
      controls.y(-1.5);
      return;
    }
    content.style.transform = `perspective(900px) rotateX(${((0.5 - y) * 3.2).toFixed(2)}deg) rotateY(${((x - 0.5) * 4.2).toFixed(2)}deg) translate3d(0, -1.5px, 0)`;
  };

  const resetPersonaTilt = (card) => {
    const content = card.firstElementChild;
    if (!content) return;
    const controls = personaControls.get(content);
    if (controls) {
      controls.rotationX(0);
      controls.rotationY(0);
      controls.y(0);
    } else {
      content.style.removeProperty("transform");
    }
  };

  const herDepthValues = nodes.herCardDepthLayers.map((layer, index) => ({
    element: layer,
    x: Number(layer.dataset.depthX) || 0,
    y: Number(layer.dataset.depthY) || 0,
    scale: Number(layer.dataset.depthScale) || 1,
    factor: index + 1,
  }));

  const updateHerParallax = (_surface, x, y) => {
    const card = nodes.herCardSurface;
    const plane = nodes.herArtPlane;
    if (!card || !plane) return;
    nodes.herProfileFrame?.classList.add("is-card-active");
    const rotationX = (0.5 - y) * 5.2;
    const rotationY = (x - 0.5) * -6.4;
    const artX = (x - 0.5) * -8;
    const artY = (y - 0.5) * -5.5;
    if (window.gsap) {
      if (!herControls) {
        window.gsap.set(card, { transformPerspective: 1100, transformOrigin: "50% 50%" });
        window.gsap.set(plane, { transformOrigin: "50% 50%" });
        herControls = {
          card: {
            y: window.gsap.quickTo(card, "y", { duration: 0.38, ease: "power3.out" }),
            scale: window.gsap.quickTo(card, "scale", { duration: 0.38, ease: "power3.out" }),
            rotationX: window.gsap.quickTo(card, "rotationX", { duration: 0.38, ease: "power3.out" }),
            rotationY: window.gsap.quickTo(card, "rotationY", { duration: 0.38, ease: "power3.out" }),
          },
          art: {
            x: window.gsap.quickTo(plane, "x", { duration: 0.48, ease: "power3.out" }),
            y: window.gsap.quickTo(plane, "y", { duration: 0.48, ease: "power3.out" }),
          },
          depth: herDepthValues.map((depth) => {
            window.gsap.set(depth.element, {
              x: depth.x,
              y: depth.y,
              scale: depth.scale,
              transformPerspective: 1100,
              transformOrigin: "50% 50%",
            });
            return {
              ...depth,
              xTo: window.gsap.quickTo(depth.element, "x", { duration: 0.5, ease: "power3.out" }),
              yTo: window.gsap.quickTo(depth.element, "y", { duration: 0.5, ease: "power3.out" }),
              rotationXTo: window.gsap.quickTo(depth.element, "rotationX", { duration: 0.5, ease: "power3.out" }),
              rotationYTo: window.gsap.quickTo(depth.element, "rotationY", { duration: 0.5, ease: "power3.out" }),
            };
          }),
        };
      }
      herControls.card.y(-2.2);
      herControls.card.scale(1.012);
      herControls.card.rotationX(rotationX);
      herControls.card.rotationY(rotationY);
      herControls.art.x(artX);
      herControls.art.y(artY);
      herControls.depth.forEach((depth) => {
        depth.xTo(depth.x + (x - 0.5) * depth.factor * 2.8);
        depth.yTo(depth.y + (y - 0.5) * depth.factor * 2.2);
        depth.rotationXTo(rotationX * (0.12 + depth.factor * 0.04));
        depth.rotationYTo(rotationY * (0.12 + depth.factor * 0.04));
      });
      return;
    }
    card.style.transform = `perspective(1100px) translate3d(0, -2.2px, 0) rotateX(${rotationX.toFixed(2)}deg) rotateY(${rotationY.toFixed(2)}deg) scale(1.012)`;
    plane.style.transform = `translate3d(${artX.toFixed(2)}px, ${artY.toFixed(2)}px, 0)`;
    herDepthValues.forEach((depth) => {
      const depthX = depth.x + (x - 0.5) * depth.factor * 2.8;
      const depthY = depth.y + (y - 0.5) * depth.factor * 2.2;
      depth.element.style.transform = `perspective(1100px) translate3d(${depthX.toFixed(2)}px, ${depthY.toFixed(2)}px, 0) rotateX(${(rotationX * (0.12 + depth.factor * 0.04)).toFixed(2)}deg) rotateY(${(rotationY * (0.12 + depth.factor * 0.04)).toFixed(2)}deg) scale(${depth.scale})`;
    });
  };

  const resetHerParallax = () => {
    const card = nodes.herCardSurface;
    const plane = nodes.herArtPlane;
    if (!card || !plane) return;
    nodes.herProfileFrame?.classList.remove("is-card-active");
    if (herControls) {
      herControls.card.y(0);
      herControls.card.scale(1);
      herControls.card.rotationX(0);
      herControls.card.rotationY(0);
      herControls.art.x(0);
      herControls.art.y(0);
      herControls.depth.forEach((depth) => {
        depth.xTo(depth.x);
        depth.yTo(depth.y);
        depth.rotationXTo(0);
        depth.rotationYTo(0);
      });
    } else {
      card.style.removeProperty("transform");
      plane.style.removeProperty("transform");
      herDepthValues.forEach((depth) => depth.element.style.removeProperty("transform"));
    }
  };

  bindSurface(nodes.composer, null);
  bindSurface(nodes.memoryField, ".memory-orb");
  bindSurface(nodes.personaDeck, ".persona-card.is-persona-visible", updatePersonaTilt, resetPersonaTilt);
  bindSurface(nodes.herProfileCard, null, updateHerParallax, resetHerParallax);
  bindSurface(nodes.mailboxList, ".mailbox-item");
}

function animateSendFeedback() {
  const icon = nodes.send?.querySelector("svg");
  if (!icon || prefersReducedMotion()) return;
  if (window.gsap) {
    window.gsap.killTweensOf(icon);
    window.gsap
      .timeline({ defaults: { ease: "power3.out" } })
      .to(icon, { y: -2, scale: 0.86, duration: 0.08 })
      .to(icon, { y: 0, scale: 1, duration: 0.2, clearProps: "transform" });
    return;
  }
  const feedback = icon.animate(
    [
      { transform: "translate3d(0, -2px, 0) scale(0.86)" },
      { transform: "translate3d(0, 0, 0) scale(1)" },
    ],
    { duration: 280, easing: "cubic-bezier(0.16, 1, 0.3, 1)" },
  );
  feedback.finished.then(() => feedback.cancel()).catch(() => {});
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
  animateSendFeedback();
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
nodes.herRetry?.addEventListener("click", () => loadHer(true));
nodes.herArtModes.forEach((button) => {
  button.addEventListener("click", () =>
    setHerArtMode(button.dataset.herArtMode, { advanceExpression: true }),
  );
});
const syncHerProfileImageState = () => {
  if (!nodes.herArtImage?.complete) return;
  nodes.herArtWindow?.classList.toggle("has-image-error", nodes.herArtImage.naturalWidth === 0);
};
nodes.herArtImage?.addEventListener("load", syncHerProfileImageState);
nodes.herArtImage?.addEventListener("error", syncHerProfileImageState);

const syncHerBackgroundImageState = () => {
  const images = [nodes.herBackdropBase, nodes.herBackdropReveal].filter(Boolean);
  const failed = images.some((image) => image.complete && image.naturalWidth === 0);
  nodes.herCanvas?.classList.toggle("has-background-error", failed);
};
[nodes.herBackdropBase, nodes.herBackdropReveal].forEach((image) => {
  image?.addEventListener("load", syncHerBackgroundImageState);
  image?.addEventListener("error", syncHerBackgroundImageState);
});
window.queueMicrotask(() => {
  syncHerProfileImageState();
  syncHerBackgroundImageState();
});
nodes.mailboxRefresh?.addEventListener("click", () => loadMailbox(true));
nodes.mailboxDetailClose?.addEventListener("click", closeMailboxDetail);
nodes.mailboxArchive?.addEventListener("click", archiveActiveMailbox);
nodes.mailboxDetail?.addEventListener("click", (event) => {
  if (event.target === nodes.mailboxDetail) closeMailboxDetail();
});

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeMemory();
    closePersonaDetail();
    closeMailboxDetail();
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
setupPointerMaterials();
setupHerReveal();
renderMessages();
autoResize();
loadStatus();
showView((window.location.hash || "#chat").slice(1));
