/// HTML and assets for the Vellum Web Companion & Mobile Reader

pub fn library_page_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>Vellum Web Companion</title>
  <style>
    :root {
      --bg: #0b0f19;
      --card-bg: #151d2f;
      --card-hover: #1e293b;
      --primary: #6366f1;
      --primary-hover: #4f46e5;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --border: rgba(255, 255, 255, 0.08);
      --badge-series: rgba(167, 139, 250, 0.15);
      --badge-series-text: #c4b5fd;
      --badge-tag: rgba(59, 130, 246, 0.15);
      --badge-tag-text: #93c5fd;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }
    header {
      background: rgba(15, 23, 42, 0.85);
      backdrop-filter: blur(12px);
      -webkit-backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--border);
      position: sticky;
      top: 0;
      z-index: 100;
      padding: 14px 20px;
    }
    .header-content {
      max-width: 1200px;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .brand-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 10px;
      font-size: 18px;
      font-weight: 700;
      letter-spacing: -0.3px;
    }
    .brand span.logo { font-size: 24px; }
    .badge-count {
      font-size: 12px;
      padding: 3px 8px;
      border-radius: 12px;
      background: rgba(99, 102, 241, 0.15);
      color: #a5b4fc;
      font-weight: 600;
    }
    .search-row {
      position: relative;
      display: flex;
      gap: 8px;
    }
    .search-input {
      width: 100%;
      padding: 10px 14px 10px 38px;
      border-radius: 10px;
      border: 1px solid var(--border);
      background: var(--card-bg);
      color: var(--text);
      font-size: 14px;
      outline: none;
      transition: border-color 0.2s;
    }
    .search-input:focus { border-color: var(--primary); }
    .search-icon {
      position: absolute;
      left: 12px;
      top: 50%;
      transform: translateY(-50%);
      font-size: 14px;
      color: var(--text-muted);
      pointer-events: none;
    }
    .chips-row {
      display: flex;
      gap: 6px;
      overflow-x: auto;
      padding-bottom: 4px;
      scrollbar-width: none;
    }
    .chips-row::-webkit-scrollbar { display: none; }
    .chip {
      font-size: 12px;
      padding: 4px 10px;
      border-radius: 14px;
      background: var(--card-bg);
      border: 1px solid var(--border);
      color: var(--text-muted);
      cursor: pointer;
      white-space: nowrap;
      transition: all 0.15s;
    }
    .chip.active {
      background: var(--primary);
      color: #ffffff;
      border-color: var(--primary);
      font-weight: 600;
    }
    main {
      flex: 1;
      max-width: 1200px;
      margin: 0 auto;
      padding: 20px;
      width: 100%;
    }
    .book-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
      gap: 16px;
    }
    @media (max-width: 640px) {
      .book-grid {
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 12px;
      }
      main { padding: 12px; }
    }
    .book-card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      overflow: hidden;
      display: flex;
      flex-direction: column;
      transition: transform 0.2s, box-shadow 0.2s, background 0.2s;
    }
    .book-card:hover {
      background: var(--card-hover);
      transform: translateY(-3px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    }
    .cover-wrap {
      width: 100%;
      aspect-ratio: 2 / 3;
      background: #1e293b;
      position: relative;
      overflow: hidden;
    }
    .cover-img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      display: block;
    }
    .cover-placeholder {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 38px;
      background: linear-gradient(135deg, #1e1b4b, #312e81);
    }
    .book-info {
      padding: 10px 12px;
      display: flex;
      flex-direction: column;
      flex: 1;
      gap: 4px;
    }
    .series-badge {
      font-size: 10px;
      font-weight: 700;
      color: var(--badge-series-text);
      background: var(--badge-series);
      padding: 2px 6px;
      border-radius: 4px;
      align-self: flex-start;
      margin-bottom: 2px;
      max-width: 100%;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .book-title {
      font-size: 13px;
      font-weight: 700;
      color: var(--text);
      line-height: 1.3;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }
    .book-author {
      font-size: 11px;
      color: var(--text-muted);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .book-actions {
      margin-top: auto;
      padding-top: 8px;
      display: flex;
      gap: 6px;
    }
    .btn {
      flex: 1;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      padding: 7px 10px;
      border-radius: 8px;
      font-size: 12px;
      font-weight: 600;
      text-decoration: none;
      cursor: pointer;
      border: none;
      transition: background 0.15s;
    }
    .btn-read {
      background: var(--primary);
      color: #ffffff;
    }
    .btn-read:hover { background: var(--primary-hover); }
    .btn-download {
      background: rgba(255, 255, 255, 0.08);
      color: var(--text);
      padding: 7px 8px;
      flex: 0 0 auto;
    }
    .btn-download:hover { background: rgba(255, 255, 255, 0.15); }
    .empty-state {
      text-align: center;
      padding: 60px 20px;
      color: var(--text-muted);
    }
    .empty-state span { font-size: 48px; display: block; margin-bottom: 12px; }
  </style>
</head>
<body>
  <header>
    <div class="header-content">
      <div class="brand-row">
        <div class="brand">
          <span class="logo">📚</span>
          <span>Vellum Companion</span>
        </div>
        <span class="badge-count" id="book-count">0 livres</span>
      </div>
      <div class="search-row">
        <span class="search-icon">🔍</span>
        <input type="text" id="search-box" class="search-input" placeholder="Rechercher par titre, auteur, saga, tag...">
      </div>
      <div class="chips-row" id="series-chips"></div>
    </div>
  </header>

  <main>
    <div class="book-grid" id="books-container">
      <div class="empty-state">
        <span>⏳</span>
        Chargement de la bibliothèque...
      </div>
    </div>
  </main>

  <script>
    let allBooks = [];
    let selectedSeries = null;

    async function loadLibrary() {
      try {
        const res = await fetch('/api/books');
        if (!res.ok) throw new Error('Impossible de charger les livres');
        allBooks = await res.json();
        renderSeriesChips();
        renderBooks();
      } catch (e) {
        document.getElementById('books-container').innerHTML = `
          <div class="empty-state">
            <span>⚠️</span>
            Erreur de connexion au serveur Vellum : ${escapeHtml(e.message)}
          </div>
        `;
      }
    }

    function renderSeriesChips() {
      const seriesSet = new Set();
      allBooks.forEach(b => {
        if (b.series && b.series.trim()) seriesSet.add(b.series.trim());
      });

      const container = document.getElementById('series-chips');
      if (seriesSet.size === 0) {
        container.style.display = 'none';
        return;
      }

      container.style.display = 'flex';
      let html = `<div class="chip ${selectedSeries === null ? 'active' : ''}" onclick="filterSeries(null)">Toutes les séries</div>`;
      seriesSet.forEach(s => {
        const active = selectedSeries === s ? 'active' : '';
        html += `<div class="chip ${active}" onclick="filterSeries('${escapeHtml(s)}')">${escapeHtml(s)}</div>`;
      });
      container.innerHTML = html;
    }

    function filterSeries(s) {
      selectedSeries = s;
      renderSeriesChips();
      renderBooks();
    }

    function renderBooks() {
      const q = document.getElementById('search-box').value.toLowerCase().trim();
      const filtered = allBooks.filter(b => {
        const matchSeries = selectedSeries ? (b.series && b.series.trim() === selectedSeries) : true;
        if (!matchSeries) return false;

        if (!q) return true;
        const inTitle = (b.title || '').toLowerCase().includes(q);
        const inAuthor = (b.author || '').toLowerCase().includes(q);
        const inSeries = (b.series || '').toLowerCase().includes(q);
        const inTags = (b.tags || []).some(t => t.toLowerCase().includes(q));
        return inTitle || inAuthor || inSeries || inTags;
      });

      document.getElementById('book-count').textContent = `${filtered.length} livre${filtered.length > 1 ? 's' : ''}`;

      const container = document.getElementById('books-container');
      if (filtered.length === 0) {
        container.innerHTML = `
          <div class="empty-state" style="grid-column: 1 / -1;">
            <span>📖</span>
            Aucun livre ne correspond à votre recherche.
          </div>
        `;
        return;
      }

      container.innerHTML = filtered.map(b => {
        const coverHtml = b.has_cover
          ? `<img class="cover-img" src="/opds/covers/${b.id}" alt="${escapeHtml(b.title)}" loading="lazy">`
          : `<div class="cover-placeholder">📖</div>`;

        let seriesBadge = '';
        if (b.series) {
          const idxStr = b.series_index != null ? ` • T.${b.series_index}` : '';
          seriesBadge = `<span class="series-badge" title="${escapeHtml(b.series)}">${escapeHtml(b.series)}${idxStr}</span>`;
        }

        const primaryFormat = (b.formats && b.formats[0]) ? b.formats[0].format.toLowerCase() : 'epub';

        return `
          <div class="book-card">
            <div class="cover-wrap">
              ${coverHtml}
            </div>
            <div class="book-info">
              ${seriesBadge}
              <div class="book-title" title="${escapeHtml(b.title)}">${escapeHtml(b.title)}</div>
              <div class="book-author" title="${escapeHtml(b.author)}">${escapeHtml(b.author)}</div>
              <div class="book-actions">
                <a class="btn btn-read" href="/read/${b.id}">📖 Lire</a>
                <a class="btn btn-download" href="/opds/download/${b.id}/${primaryFormat}" title="Télécharger (${primaryFormat.toUpperCase()})">⬇️</a>
              </div>
            </div>
          </div>
        `;
      }).join('');
    }

    function escapeHtml(str) {
      if (!str) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
    }

    document.getElementById('search-box').addEventListener('input', renderBooks);
    window.addEventListener('DOMContentLoaded', loadLibrary);
  </script>
</body>
</html>"#
}

pub fn reader_page_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>Lecteur Vellum</title>
  <style>
    :root {
      --bg: #0f172a;
      --text: #e2e8f0;
      --nav-bg: rgba(15, 23, 42, 0.95);
      --border: rgba(255, 255, 255, 0.1);
      --primary: #6366f1;
      --primary-hover: #4f46e5;
      --font-size: 19px;
      --line-height: 1.8;
      --max-width: 740px;
    }
    body.theme-light {
      --bg: #f8fafc;
      --text: #0f172a;
      --nav-bg: rgba(255, 255, 255, 0.95);
      --border: rgba(0, 0, 0, 0.1);
      --primary: #4f46e5;
    }
    body.theme-sepia {
      --bg: #fbf0d9;
      --text: #453221;
      --nav-bg: rgba(251, 240, 217, 0.96);
      --border: rgba(69, 50, 33, 0.15);
      --primary: #92400e;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Georgia, serif;
      font-size: var(--font-size);
      line-height: var(--line-height);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      transition: background 0.2s, color 0.2s;
    }
    /* Top Bar */
    header {
      background: var(--nav-bg);
      backdrop-filter: blur(12px);
      -webkit-backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--border);
      position: sticky;
      top: 0;
      z-index: 100;
      padding: 10px 16px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      user-select: none;
    }
    .nav-left, .nav-right {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .btn-icon {
      background: transparent;
      border: 1px solid var(--border);
      color: var(--text);
      border-radius: 8px;
      padding: 6px 10px;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      text-decoration: none;
      transition: all 0.15s;
    }
    .btn-icon:hover {
      border-color: var(--primary);
      color: var(--primary);
    }
    .chapter-title-header {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      max-width: 380px;
      text-align: center;
      flex: 1;
    }
    @media (max-width: 600px) {
      .chapter-title-header { max-width: 140px; font-size: 11px; }
      .btn-icon span.label { display: none; }
    }
    /* Reading Container */
    main {
      flex: 1;
      width: 100%;
      max-width: var(--max-width);
      margin: 0 auto;
      padding: 32px 20px 80px 20px;
    }
    #reader-content {
      word-break: break-word;
    }
    #reader-content img {
      max-width: 100%;
      height: auto;
      display: block;
      margin: 16px auto;
      border-radius: 6px;
    }
    #reader-content p {
      margin-bottom: 1.4em;
      text-align: justify;
    }
    #reader-content h1, #reader-content h2, #reader-content h3 {
      margin: 1.8em 0 0.8em 0;
      line-height: 1.3;
    }
    /* Bottom Navigation Bar */
    footer {
      background: var(--nav-bg);
      backdrop-filter: blur(12px);
      -webkit-backdrop-filter: blur(12px);
      border-top: 1px solid var(--border);
      position: sticky;
      bottom: 0;
      z-index: 100;
      padding: 10px 16px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      user-select: none;
    }
    .progress-text {
      font-size: 12px;
      font-weight: 600;
      color: var(--text);
      opacity: 0.8;
    }
    /* TOC Drawer */
    #toc-drawer {
      position: fixed;
      top: 0;
      left: 0;
      bottom: 0;
      width: 320px;
      max-width: 85vw;
      background: var(--nav-bg);
      border-right: 1px solid var(--border);
      z-index: 200;
      transform: translateX(-100%);
      transition: transform 0.25s ease-out;
      display: flex;
      flex-direction: column;
      box-shadow: 4px 0 24px rgba(0,0,0,0.4);
    }
    #toc-drawer.open {
      transform: translateX(0);
    }
    .drawer-header {
      padding: 16px;
      border-bottom: 1px solid var(--border);
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-weight: 700;
      font-size: 15px;
    }
    .drawer-body {
      flex: 1;
      overflow-y: auto;
      padding: 8px;
    }
    .toc-item {
      padding: 10px 12px;
      border-radius: 8px;
      font-size: 14px;
      cursor: pointer;
      color: var(--text);
      transition: background 0.15s;
      display: block;
      text-decoration: none;
      line-height: 1.4;
    }
    .toc-item:hover { background: rgba(255, 255, 255, 0.06); }
    .toc-item.active {
      background: var(--primary);
      color: #ffffff;
      font-weight: 600;
    }
    #toc-backdrop {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.6);
      z-index: 190;
      display: none;
    }
    #toc-backdrop.open { display: block; }
  </style>
</head>
<body class="theme-dark">
  <div id="toc-backdrop" onclick="toggleToc(false)"></div>

  <div id="toc-drawer">
    <div class="drawer-header">
      <span>📑 Table des matières</span>
      <button class="btn-icon" onclick="toggleToc(false)">✕</button>
    </div>
    <div class="drawer-body" id="toc-list"></div>
  </div>

  <header>
    <div class="nav-left">
      <a class="btn-icon" href="/"><span class="label">← Bibliothèque</span><span style="font-size:18px">📚</span></a>
      <button class="btn-icon" onclick="toggleToc(true)">📑 <span class="label">TOC</span></button>
    </div>
    <div class="chapter-title-header" id="header-chapter-title">Chargement...</div>
    <div class="nav-right">
      <button class="btn-icon" onclick="changeFontSize(-1)">A-</button>
      <button class="btn-icon" onclick="changeFontSize(1)">A+</button>
      <button class="btn-icon" onclick="cycleTheme()" title="Changer le thème">🌓</button>
    </div>
  </header>

  <main>
    <div id="reader-content">
      <p style="text-align:center; padding: 40px 0; color: gray;">Chargement du livre...</p>
    </div>
  </main>

  <footer>
    <button class="btn-icon" id="btn-prev" onclick="goToChapter(currentChapter - 1)">◀ <span class="label">Précédent</span></button>
    <div class="progress-text" id="progress-indicator">- / -</div>
    <button class="btn-icon" id="btn-next" onclick="goToChapter(currentChapter + 1)"><span class="label">Suivant</span> ▶</button>
  </footer>

  <script>
    const pathParts = window.location.pathname.split('/');
    const bookId = pathParts[pathParts.length - 1] || pathParts[pathParts.length - 2];

    let currentChapter = 0;
    let totalChapters = 0;
    let tocEntries = [];
    let currentFontSize = parseInt(localStorage.getItem('vellum_reader_fs') || '19');
    let currentTheme = localStorage.getItem('vellum_reader_theme') || 'dark';

    function applyTheme(theme) {
      document.body.className = `theme-${theme}`;
      localStorage.setItem('vellum_reader_theme', theme);
      currentTheme = theme;
    }

    function cycleTheme() {
      const themes = ['dark', 'sepia', 'light'];
      const nextIdx = (themes.indexOf(currentTheme) + 1) % themes.length;
      applyTheme(themes[nextIdx]);
    }

    function changeFontSize(delta) {
      currentFontSize = Math.min(32, Math.max(14, currentFontSize + delta));
      document.documentElement.style.setProperty('--font-size', `${currentFontSize}px`);
      localStorage.setItem('vellum_reader_fs', currentFontSize.toString());
    }

    function toggleToc(show) {
      document.getElementById('toc-drawer').classList.toggle('open', show);
      document.getElementById('toc-backdrop').classList.toggle('open', show);
    }

    async function initReader() {
      applyTheme(currentTheme);
      document.documentElement.style.setProperty('--font-size', `${currentFontSize}px`);

      try {
        // Load TOC
        const tocRes = await fetch(`/api/books/${bookId}/toc`);
        if (tocRes.ok) {
          tocEntries = await tocRes.json();
          renderToc();
        }

        // Load saved position or chapter 0
        const savedCh = localStorage.getItem('vellum_pos_' + bookId);
        const startCh = savedCh !== null ? parseInt(savedCh) : 0;
        await goToChapter(startCh);
      } catch (e) {
        document.getElementById('reader-content').innerHTML = `
          <p style="color:red; text-align:center;">Erreur de chargement: ${escapeHtml(e.message)}</p>
        `;
      }
    }

    function renderToc() {
      const list = document.getElementById('toc-list');
      if (!tocEntries || tocEntries.length === 0) {
        list.innerHTML = '<p style="padding:16px; color:gray;">Aucune table des matières disponible.</p>';
        return;
      }
      list.innerHTML = tocEntries.map((item, idx) => `
        <div class="toc-item ${idx === currentChapter ? 'active' : ''}" onclick="goToChapter(${item.index}); toggleToc(false);">
          ${escapeHtml(item.title)}
        </div>
      `).join('');
    }

    async function goToChapter(idx) {
      if (idx < 0) return;
      if (totalChapters > 0 && idx >= totalChapters) return;

      document.getElementById('reader-content').innerHTML = `
        <p style="text-align:center; padding: 40px 0; color: gray;">Chargement...</p>
      `;

      try {
        const res = await fetch(`/api/books/${bookId}/chapter/${idx}`);
        if (!res.ok) throw new Error('Chapitre introuvable');
        const data = await res.json();

        currentChapter = data.chapter_index;
        totalChapters = data.total_chapters;

        document.getElementById('header-chapter-title').textContent = data.title || `Chapitre ${currentChapter + 1}`;
        document.getElementById('reader-content').innerHTML = data.html;
        document.getElementById('progress-indicator').textContent = `${currentChapter + 1} / ${totalChapters}`;

        document.getElementById('btn-prev').disabled = currentChapter === 0;
        document.getElementById('btn-next').disabled = currentChapter + 1 >= totalChapters;

        localStorage.setItem('vellum_pos_' + bookId, currentChapter.toString());
        renderToc();
        window.scrollTo(0, 0);

        // Sync progress with backend silently
        fetch(`/api/books/${bookId}/progress`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            chapter_index: currentChapter,
            percentage: totalChapters > 0 ? (currentChapter + 1) / totalChapters : 0
          })
        }).catch(() => {});

      } catch (e) {
        document.getElementById('reader-content').innerHTML = `
          <p style="color:red; text-align:center;">Erreur : ${escapeHtml(e.message)}</p>
        `;
      }
    }

    function escapeHtml(str) {
      if (!str) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
    }

    // Keyboard navigation
    window.addEventListener('keydown', (e) => {
      if (e.key === 'ArrowRight' || e.key === 'PageDown') {
        goToChapter(currentChapter + 1);
      } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        goToChapter(currentChapter - 1);
      }
    });

    window.addEventListener('DOMContentLoaded', initReader);
  </script>
</body>
</html>"#
}

