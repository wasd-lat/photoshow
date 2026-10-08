const MANIFEST_URL = "./docs/manifest.json";

const nav = document.querySelector("#docNav");
const content = document.querySelector("#content");
const pageToc = document.querySelector("#pageToc");
const search = document.querySelector("#search");
const searchResults = document.querySelector("#searchResults");
const sidebar = document.querySelector("#sidebar");
const collapseAll = document.querySelector("#collapseAll");
const themeToggle = document.querySelector("#themeToggle");

const cache = new Map();

let manifest = null;
let fuse = null;
let FuseCtor = null;
let searchDocuments = [];
let currentPath = null;
let tocObserver = null;
let keyboardResultIndex = -1;

const domainIcons = {
  philosophy: "ph-compass",
  media: "ph-image-square",
  editor: "ph-sliders-horizontal",
  application: "ph-gear",
  interface: "ph-layout",
  persistence: "ph-database",
  release: "ph-package",
  agents: "ph-robot"
};

const THEME_STORAGE_KEY = "photoshow-docs-theme";

function applyTheme(theme, persist) {
  const nextTheme = theme === "dark" ? "dark" : "light";
  const isDark = nextTheme === "dark";

  document.documentElement.dataset.theme = nextTheme;
  document.documentElement.style.colorScheme = nextTheme;

  if (themeToggle) {
    const label = isDark ? "Ativar tema claro" : "Ativar tema escuro";
    const icon = themeToggle.querySelector(".ph");

    themeToggle.setAttribute("aria-label", label);
    themeToggle.setAttribute("title", label);
    themeToggle.setAttribute("aria-pressed", String(isDark));

    if (icon) {
      icon.className = isDark ? "ph ph-sun" : "ph ph-moon";
    }
  }

  if (persist) {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, nextTheme);
    } catch (_) {}
  }
}

function initializeTheme() {
  applyTheme(document.documentElement.dataset.theme || "light", false);
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, function (char) {
    return {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;"
    }[char];
  });
}

function slugify(value) {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

function stripMarkdown(markdown) {
  return markdown
    .replace(/\x60\x60\x60[\s\S]*?\x60\x60\x60/g, " ")
    .replace(/\x60([^\x60]+)\x60/g, "$1")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[#>*_~|]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function sourceHeadings(markdown) {
  return markdown
    .split("\n")
    .map(function (line) {
      const match = /^(#{2,3})\s+(.+)$/.exec(line);
      if (!match) return null;
      const title = match[2].replace(/[*\x60]/g, "").trim();
      return { level: match[1].length, title: title, id: slugify(title) };
    })
    .filter(Boolean);
}

async function loadText(path) {
  if (cache.has(path)) return cache.get(path);

  const response = await fetch("./docs/" + path);
  if (!response.ok) {
    throw new Error("Falha ao carregar " + path + " (" + response.status + ")");
  }

  const text = await response.text();
  cache.set(path, text);
  return text;
}

function fileMeta(path) {
  for (const domain of manifest.domains) {
    const file = domain.files.find(function (candidate) {
      return candidate.path === path;
    });
    if (file) return { domain: domain, file: file };
  }

  if (path === "home.md") {
    return {
      domain: { id: "home", title: "Documentação" },
      file: { title: "Início", path: path }
    };
  }

  if (path === "about.md") {
    return {
      domain: { id: "project", title: "Projeto" },
      file: { title: "Sobre", path: path }
    };
  }

  return null;
}

function closeMobileNavigation() {
  window.dispatchEvent(new CustomEvent("close-nav"));
}

function buildSearchIndex() {
  const documents = [];

  const home = cache.get("home.md") || "";
  documents.push({
    path: "home.md",
    title: "Início",
    domain: "Documentação",
    headings: sourceHeadings(home).map(function (item) {
      return item.title;
    }).join(" "),
    content: stripMarkdown(home)
  });

  for (const domain of manifest.domains) {
    for (const file of domain.files) {
      const markdown = cache.get(file.path) || "";
      documents.push({
        path: file.path,
        title: file.title,
        domain: domain.title,
        headings: sourceHeadings(markdown).map(function (item) {
          return item.title;
        }).join(" "),
        content: stripMarkdown(markdown)
      });
    }
  }

  const about = cache.get("about.md") || "";
  documents.push({
    path: "about.md",
    title: "Sobre",
    domain: "Projeto",
    headings: sourceHeadings(about).map(function (item) {
      return item.title;
    }).join(" "),
    content: stripMarkdown(about)
  });

  searchDocuments = documents;

  if (!FuseCtor) {
    fuse = null;
    return;
  }

  fuse = new FuseCtor(documents, {
    keys: [
      { name: "title", weight: 0.38 },
      { name: "headings", weight: 0.30 },
      { name: "domain", weight: 0.12 },
      { name: "content", weight: 0.20 }
    ],
    threshold: 0.34,
    distance: 120,
    ignoreLocation: true,
    includeScore: true,
    minMatchCharLength: 2
  });
}

async function loadFuse() {
  try {
    const module = await import("https://cdn.jsdelivr.net/npm/fuse.js@7.5.0/dist/fuse.min.mjs");
    FuseCtor = module.default || module.Fuse || null;
  } catch (error) {
    FuseCtor = null;
    console.warn("Fuse.js indisponível; usando busca textual de fallback.", error);
  }
}

function excerptFor(item, query) {
  const text = item.content;
  if (!text) return item.domain;

  const normalizedText = text.toLocaleLowerCase("pt-BR");
  const normalizedQuery = query.toLocaleLowerCase("pt-BR");
  const position = normalizedText.indexOf(normalizedQuery);

  if (position >= 0) {
    const start = Math.max(0, position - 58);
    const end = Math.min(text.length, position + query.length + 105);
    return (start > 0 ? "…" : "") +
      text.slice(start, end) +
      (end < text.length ? "…" : "");
  }

  return text.length > 160 ? text.slice(0, 160) + "…" : text;
}

function resetKeyboardSearchSelection() {
  keyboardResultIndex = -1;
  document.querySelectorAll(".search-result").forEach(function (item) {
    item.classList.remove("is-keyboard-active");
  });
}

function renderSearchResults(query) {
  const cleanQuery = query.trim();

  if (cleanQuery.length < 2) {
    searchResults.hidden = true;
    searchResults.innerHTML = "";
    resetKeyboardSearchSelection();
    return;
  }

  const results = fuse
    ? fuse.search(cleanQuery, { limit: 10 })
    : searchDocuments
        .filter(function (item) {
          const haystack = [
            item.title,
            item.domain,
            item.headings,
            item.content
          ].join(" ").toLocaleLowerCase("pt-BR");

          return haystack.includes(cleanQuery.toLocaleLowerCase("pt-BR"));
        })
        .slice(0, 10)
        .map(function (item) {
          return { item: item, score: 0 };
        });

  if (!results.length) {
    searchResults.innerHTML =
      '<div class="search-empty">' +
      '<i class="ph ph-magnifying-glass" aria-hidden="true"></i> ' +
      "Nenhum resultado para “" + escapeHtml(cleanQuery) + "”." +
      "</div>";
    searchResults.hidden = false;
    resetKeyboardSearchSelection();
    return;
  }

  searchResults.innerHTML = results.map(function (result) {
    const item = result.item;
    return (
      '<a class="search-result" role="option" href="#/docs/' + item.path + '">' +
        '<span class="search-result-icon">' +
          '<i class="ph ph-file-text" aria-hidden="true"></i>' +
        "</span>" +
        "<span>" +
          "<strong>" + escapeHtml(item.title) + "</strong>" +
          "<small>" +
            escapeHtml(item.domain) + " · " +
            escapeHtml(excerptFor(item, cleanQuery)) +
          "</small>" +
        "</span>" +
      "</a>"
    );
  }).join("");

  searchResults.hidden = false;
  resetKeyboardSearchSelection();
}

function moveSearchSelection(delta) {
  const items = Array.from(searchResults.querySelectorAll(".search-result"));
  if (!items.length || searchResults.hidden) return;

  keyboardResultIndex = Math.max(
    0,
    Math.min(items.length - 1, keyboardResultIndex + delta)
  );

  items.forEach(function (item, index) {
    item.classList.toggle("is-keyboard-active", index === keyboardResultIndex);
  });

  items[keyboardResultIndex].scrollIntoView({ block: "nearest" });
}

function setActiveFile(path) {
  document.querySelectorAll(".file-overview").forEach(function (link) {
    const active = link.dataset.path === path;
    link.classList.toggle("active", active);

    if (active) {
      link.setAttribute("aria-current", "page");
      const fileDetails = link.closest(".nav-file");
      const domainDetails = link.closest(".nav-domain");
      if (fileDetails) fileDetails.open = true;
      if (domainDetails) domainDetails.open = true;
    } else {
      link.removeAttribute("aria-current");
    }
  });
}

function setActiveTopic(topicId) {
  document.querySelectorAll(".topic-link").forEach(function (link) {
    link.classList.toggle(
      "active",
      link.dataset.path === currentPath && link.dataset.topic === topicId
    );
  });

  document.querySelectorAll("#pageToc a").forEach(function (link) {
    link.classList.toggle("active", link.dataset.topic === topicId);
  });
}

function buildPageToc() {
  const headings = Array.from(
    content.querySelectorAll(".markdown-body h2, .markdown-body h3")
  );

  pageToc.innerHTML = headings.map(function (heading) {
    const level = heading.tagName === "H2" ? 2 : 3;
    const label = heading.dataset.cleanTitle || heading.textContent.trim();
    return (
      '<a data-level="' + level + '"' +
      ' data-topic="' + heading.id + '"' +
      ' href="#/docs/' + currentPath + "#" + heading.id + '">' +
      escapeHtml(label) +
      "</a>"
    );
  }).join("");

  if (tocObserver) tocObserver.disconnect();

  tocObserver = new IntersectionObserver(function (entries) {
    const visible = entries
      .filter(function (entry) {
        return entry.isIntersecting;
      })
      .sort(function (a, b) {
        return a.boundingClientRect.top - b.boundingClientRect.top;
      });

    if (visible.length) {
      setActiveTopic(visible[0].target.id);
    }
  }, {
    rootMargin: "-88px 0px -68% 0px",
    threshold: 0
  });

  headings.forEach(function (heading) {
    tocObserver.observe(heading);
  });
}

function enhanceRenderedMarkdown() {
  const usedIds = new Map();

  content.querySelectorAll(
    ".markdown-body h1, .markdown-body h2, .markdown-body h3"
  ).forEach(function (heading) {
    const cleanTitle = heading.textContent.trim();
    const base = slugify(cleanTitle) || "secao";
    const count = usedIds.get(base) || 0;

    usedIds.set(base, count + 1);
    heading.id = count ? base + "-" + (count + 1) : base;
    heading.dataset.cleanTitle = cleanTitle;

    if (heading.tagName !== "H1") {
      const anchor = document.createElement("a");
      anchor.className = "heading-anchor";
      anchor.href = "#/docs/" + currentPath + "#" + heading.id;
      anchor.setAttribute("aria-label", "Link para " + cleanTitle);
      anchor.innerHTML = '<i class="ph ph-link" aria-hidden="true"></i>';
      heading.append(anchor);
    }
  });

  content.querySelectorAll('.markdown-body a[href^="http"]').forEach(function (link) {
    link.target = "_blank";
    link.rel = "noreferrer";
  });

  content.querySelectorAll(".markdown-body table").forEach(function (table) {
    const wrapper = document.createElement("div");
    wrapper.className = "table-scroll";
    table.parentNode.insertBefore(wrapper, table);
    wrapper.append(table);
  });

  content.querySelectorAll(".markdown-body pre").forEach(function (pre) {
    const code = pre.querySelector("code");
    if (!code) return;

    const button = document.createElement("button");
    button.className = "code-copy";
    button.type = "button";
    button.setAttribute("aria-label", "Copiar código");
    button.innerHTML = '<i class="ph ph-copy" aria-hidden="true"></i>';

    button.addEventListener("click", async function () {
      try {
        await navigator.clipboard.writeText(code.textContent || "");
        button.innerHTML = '<i class="ph ph-check" aria-hidden="true"></i>';
        button.setAttribute("aria-label", "Código copiado");

        setTimeout(function () {
          button.innerHTML = '<i class="ph ph-copy" aria-hidden="true"></i>';
          button.setAttribute("aria-label", "Copiar código");
        }, 1300);
      } catch {
        button.setAttribute("aria-label", "Não foi possível copiar");
      }
    });

    pre.append(button);
  });
}

async function openDoc(path, anchor) {
  try {
    currentPath = path;
    const markdown = await loadText(path);
    const meta = fileMeta(path);

    if (!window.marked) {
      throw new Error("Marked não foi carregado pelo CDN.");
    }

    content.innerHTML =
      '<div class="doc-context">' +
        '<i class="ph ph-book-open-text" aria-hidden="true"></i>' +
        "<span>" + escapeHtml(meta ? meta.domain.title : "Documentação") + "</span>" +
      "</div>" +
      '<article class="markdown-body">' +
        marked.parse(markdown, { gfm: true, breaks: false }) +
      "</article>";

    enhanceRenderedMarkdown();
    buildPageToc();
    setActiveFile(path);

    document.title =
      (meta ? meta.file.title : "Documentação") + " — PhotoShow";

    if (anchor) {
      requestAnimationFrame(function () {
        const target = document.getElementById(anchor);
        if (target) target.scrollIntoView({ block: "start" });
        setActiveTopic(anchor);
      });
    } else {
      window.scrollTo({ top: 0, behavior: "auto" });
    }

    content.focus({ preventScroll: true });
    closeMobileNavigation();
  } catch (error) {
    content.innerHTML =
      '<div class="error-card">' +
        "<strong>Não foi possível carregar a página.</strong>" +
        "<p>" + escapeHtml(error.message) + "</p>" +
        "<p>Sirva a pasta website/ por HTTP; navegadores normalmente bloqueiam fetch() em file://.</p>" +
      "</div>";
  }
}

async function buildNavigation() {
  nav.innerHTML = "";

  const homeLink = document.createElement("a");
  homeLink.className = "nav-home file-overview";
  homeLink.href = "#/docs/home.md";
  homeLink.dataset.path = "home.md";
  homeLink.innerHTML =
    '<i class="ph ph-house" aria-hidden="true"></i>' +
    '<span>Início</span>';
  nav.append(homeLink);

  for (const domain of manifest.domains) {
    const domainDetails = document.createElement("details");
    domainDetails.className = "nav-domain";
    domainDetails.open = false;

    const domainSummary = document.createElement("summary");
    domainSummary.innerHTML =
      '<i class="ph ' +
      (domainIcons[domain.id] || "ph-folder") +
      ' nav-domain-icon" aria-hidden="true"></i>' +
      "<span>" + escapeHtml(domain.title) + "</span>" +
      '<i class="ph ph-caret-right nav-caret" aria-hidden="true"></i>';

    const domainChildren = document.createElement("div");
    domainChildren.className = "nav-domain-children";

    for (const file of domain.files) {
      const markdown = await loadText(file.path);
      const fileDetails = document.createElement("details");
      fileDetails.className = "nav-file";

      const fileSummary = document.createElement("summary");
      fileSummary.innerHTML =
        '<i class="ph ph-caret-right nav-caret" aria-hidden="true"></i>' +
        '<span class="nav-file-title">' + escapeHtml(file.title) + "</span>";

      const fileChildren = document.createElement("div");
      fileChildren.className = "nav-file-children";

      const overview = document.createElement("a");
      overview.className = "file-overview";
      overview.href = "#/docs/" + file.path;
      overview.dataset.path = file.path;
      overview.textContent = "Visão geral";
      fileChildren.append(overview);

      sourceHeadings(markdown)
        .filter(function (item) {
          return item.level === 2;
        })
        .forEach(function (item) {
          const link = document.createElement("a");
          link.className = "topic-link";
          link.href = "#/docs/" + file.path + "#" + item.id;
          link.dataset.path = file.path;
          link.dataset.topic = item.id;
          link.textContent = item.title;
          fileChildren.append(link);
        });

      fileDetails.append(fileSummary, fileChildren);
      domainChildren.append(fileDetails);
    }

    domainDetails.append(domainSummary, domainChildren);
    nav.append(domainDetails);
  }
}

async function preloadDocumentation() {
  const paths = manifest.domains.flatMap(function (domain) {
    return domain.files.map(function (file) {
      return file.path;
    });
  });

  paths.push("home.md", "about.md");
  await Promise.all(paths.map(loadText));
}

function route() {
  const raw = location.hash || "#/docs/home.md";
  const match = /^#\/docs\/([^#]+)(?:#(.+))?$/.exec(raw);

  if (!match) {
    openDoc("home.md");
    return;
  }

  openDoc(match[1], match[2]);
}

async function initialize() {
  try {
    const response = await fetch(MANIFEST_URL);
    if (!response.ok) {
      throw new Error("Manifesto da documentação indisponível.");
    }

    manifest = await response.json();

    await preloadDocumentation();
    await buildNavigation();

    // A documentação deve abrir mesmo se o CDN de busca estiver lento ou indisponível.
    buildSearchIndex();
    route();

    loadFuse().then(function () {
      buildSearchIndex();
    });
  } catch (error) {
    content.innerHTML =
      '<div class="error-card">' +
        "<strong>Falha ao iniciar a documentação.</strong>" +
        "<p>" + escapeHtml(error.message) + "</p>" +
      "</div>";
  }
}

search.addEventListener("input", function () {
  renderSearchResults(search.value || "");
});

search.addEventListener("wa-clear", function () {
  renderSearchResults("");
});

search.addEventListener("keydown", function (event) {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    moveSearchSelection(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    moveSearchSelection(-1);
  } else if (event.key === "Enter" && keyboardResultIndex >= 0) {
    const items = Array.from(searchResults.querySelectorAll(".search-result"));
    if (items[keyboardResultIndex]) items[keyboardResultIndex].click();
  } else if (event.key === "Escape") {
    searchResults.hidden = true;
    keyboardResultIndex = -1;
    search.blur();
  }
});

searchResults.addEventListener("click", function (event) {
  if (event.target.closest(".search-result")) {
    searchResults.hidden = true;
    search.value = "";
    keyboardResultIndex = -1;
  }
});

document.addEventListener("keydown", function (event) {
  if (
    event.key === "/" &&
    document.activeElement !== search &&
    !["INPUT", "TEXTAREA"].includes(
      document.activeElement ? document.activeElement.tagName : ""
    )
  ) {
    event.preventDefault();
    search.focus();
  }
});

document.addEventListener("click", function (event) {
  if (!event.target.closest(".search-wrap")) {
    searchResults.hidden = true;
    keyboardResultIndex = -1;
  }
});

collapseAll.addEventListener("click", function () {
  document.querySelectorAll(".doc-nav details").forEach(function (details) {
    details.open = false;
  });
});

if (themeToggle) {
  themeToggle.addEventListener("click", function () {
    const current = document.documentElement.dataset.theme || "light";
    applyTheme(current === "dark" ? "light" : "dark", true);
  });
}

initializeTheme();

window.addEventListener("hashchange", route);
window.addEventListener("DOMContentLoaded", initialize);
