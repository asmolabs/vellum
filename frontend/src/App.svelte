<script>
  import { onMount } from 'svelte';
  import ConfirmDialog from './lib/ConfirmDialog.svelte';
  import EditDialog from './lib/EditDialog.svelte';

  // App State (Svelte 5 Runes)
  let books = $state([]);
  let libraryPath = $state('');
  let searchQuery = $state('');
  let selectedBook = $state(null);
  let selectedTag = $state(null);
  let allTags = $state([]);
  let viewMode = $state('grid');
  let plugins = $state([]);
  let isPluginRunning = $state(false);

  // Pagination State
  let skip = $state(0);
  const limit = 20;
  let hasMore = $state(true);
  let isLoading = $state(false);
  let totalBooksCount = $state(0);
  let tagCounts = $state({});

  // EPUB Reader State
  let readingBook = $state(null);
  let epubBook = null;
  let rendition = null;

  // Dialogs State
  let showConfirmDelete = $state(false);
  let confirmDeleteResolve = null;
  let showEditDialog = $state(false);

  // Search Timeout (Debounce)
  let searchTimeout;

  // DOM bindings
  let fileInput;
  let booksContainer;

  onMount(() => {
    loadLibraryPath();
    loadTags();
    loadBooks(true);
    loadPlugins();
  });

  async function loadLibraryPath() {
    try {
      const res = await fetch('/books/library-path');
      const data = await res.json();
      libraryPath = data.path;
    } catch (err) {
      console.error(err);
    }
  }

  async function loadTags() {
    try {
      const res = await fetch('/books/tags');
      const data = await res.json();
      totalBooksCount = data.total;
      tagCounts = data.tags;
      allTags = Object.keys(tagCounts);
    } catch (err) {
      console.error(err);
    }
  }

  async function loadBooks(reset = false) {
    if (isLoading) return;
    
    if (reset) {
      skip = 0;
      books = [];
      hasMore = true;
    }
    
    if (!hasMore) return;
    
    isLoading = true;
    
    try {
      let url = `/books?skip=${skip}&limit=${limit}`;
      if (searchQuery) {
        url += `&search=${encodeURIComponent(searchQuery)}`;
      }
      if (selectedTag) {
        url += `&tag=${encodeURIComponent(selectedTag)}`;
      }
      
      const res = await fetch(url);
      const newBooks = await res.json();
      
      if (newBooks.length < limit) {
        hasMore = false;
      }
      
      if (reset) {
        books = newBooks;
      } else {
        books = [...books, ...newBooks];
      }
      skip = books.length;
    } catch (err) {
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  async function loadPlugins() {
    try {
      const res = await fetch('/books/plugins/list');
      plugins = await res.json();
    } catch (err) {
      console.error(err);
    }
  }

  function handleSearch(e) {
    searchQuery = e.target.value;
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      loadBooks(true);
    }, 300);
  }

  function filterByTag(tag) {
    selectedTag = tag;
    loadBooks(true);
  }

  async function changeLibraryPath() {
    if (window.pywebview && window.pywebview.api && window.pywebview.api.select_directory) {
      try {
        const chosenPath = await window.pywebview.api.select_directory();
        if (chosenPath) {
          const res = await fetch('/books/library-path', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ path: chosenPath })
          });
          const data = await res.json();
          libraryPath = data.path;
          selectedBook = null;
          await loadTags();
          loadBooks(true);
        }
      } catch (err) {
        alert("Erreur lors de la sélection du dossier: " + err);
      }
    } else {
      const path = prompt("Entrez le chemin absolu du dossier de bibliothèque :", libraryPath);
      if (path) {
        const res = await fetch('/books/library-path', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path })
        });
        const data = await res.json();
        libraryPath = data.path;
        selectedBook = null;
        await loadTags();
        loadBooks(true);
      }
    }
  }

  async function handleImport(e) {
    const file = e.target.files[0];
    if (!file) return;

    const formData = new FormData();
    formData.append('file', file);

    try {
      const res = await fetch('/books/import', {
        method: 'POST',
        body: formData
      });
      if (!res.ok) throw new Error(await res.text());
      const newBook = await res.json();
      await loadTags();
      await loadBooks(true);
      selectedBook = newBook;
    } catch (err) {
      alert("Erreur d'importation : " + err.message);
    } finally {
      fileInput.value = '';
    }
  }

  function confirmDelete() {
    return new Promise((resolve) => {
      showConfirmDelete = true;
      confirmDeleteResolve = resolve;
    });
  }

  async function handleDeleteBook(id) {
    const confirmed = await confirmDelete();
    if (!confirmed) return;
    
    try {
      const res = await fetch(`/books/${id}`, {
        method: 'DELETE'
      });
      if (!res.ok) throw new Error(await res.text());
      selectedBook = null;
      await loadTags();
      await loadBooks(true);
    } catch (err) {
      alert("Échec de la suppression: " + err.message);
    }
  }

  async function handleSaveMetadata(data, file) {
    const id = selectedBook.id;

    try {
      let res = await fetch(`/books/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
      });
      if (!res.ok) throw new Error(await res.text());
      let updatedBook = await res.json();

      if (file) {
        const formData = new FormData();
        formData.append('cover', file);
        res = await fetch(`/books/${id}/cover`, {
          method: 'POST',
          body: formData
        });
        if (!res.ok) throw new Error(await res.text());
        updatedBook = await res.json();
      }

      showEditDialog = false;
      await loadTags();
      await loadBooks(true);
      // Refresh details
      selectedBook = books.find(b => b.id === updatedBook.id) || updatedBook;
    } catch (err) {
      alert("Échec de la mise à jour : " + err.message);
    }
  }

  async function applyPlugin(bookId, pluginId) {
    isPluginRunning = true;
    
    try {
      const res = await fetch(`/books/${bookId}/plugin/${pluginId}`, {
        method: 'POST'
      });
      if (!res.ok) throw new Error(await res.text());
      const newBook = await res.json();
      isPluginRunning = false;
      alert("Plugin exécuté avec succès !");
      await loadTags();
      await loadBooks(true);
      selectedBook = books.find(b => b.id === newBook.id) || newBook;
    } catch (err) {
      isPluginRunning = false;
      alert("Échec de l'exécution du plugin : " + err.message);
    }
  }

  // EPUB Reader Actions
  async function readBook(book) {
    const epubFormat = book.formats.find(f => f.format === 'epub');
    if (!epubFormat) {
      alert("La liseuse intégrée ne prend en charge que le format EPUB pour l'instant.");
      return;
    }

    readingBook = book;

    try {
      const res = await fetch(`/books/${book.id}/file/epub`);
      const buffer = await res.arrayBuffer();

      if (epubBook) {
        epubBook.destroy();
      }
      
      // Wait for DOM layout
      setTimeout(async () => {
        const readerArea = document.getElementById('reader-area');
        if (readerArea) readerArea.innerHTML = '';

        // ePub is exposed from CDN in index.html
        epubBook = window.ePub(buffer);
        rendition = epubBook.renderTo('reader-area', {
          width: '100%',
          height: '100%',
          flow: 'paginated'
        });

        await rendition.display();
      }, 50);
    } catch (err) {
      alert("Erreur de chargement du livre: " + err.message);
      closeReader();
    }
  }

  function closeReader() {
    if (rendition) {
      rendition.destroy();
      rendition = null;
    }
    if (epubBook) {
      epubBook.destroy();
      epubBook = null;
    }
    readingBook = null;
  }

  function prevPage() {
    if (rendition) rendition.prev();
  }

  function nextPage() {
    if (rendition) rendition.next();
  }

  // Handle scroll for infinite loading
  function handleScroll() {
    if (!booksContainer) return;
    if (booksContainer.scrollHeight - booksContainer.scrollTop - booksContainer.clientHeight < 100) {
      if (hasMore && !isLoading) {
        loadBooks(false);
      }
    }
  }
</script>

<div class="app-container">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="logo-section">
      <span class="logo-icon">📚</span>
      <h1 class="logo-text">Bibliothèque</h1>
    </div>

    <div class="import-btn-container">
      <label class="import-label">
        <span>➕ Importer un Livre</span>
        <input type="file" accept=".epub,.mobi,.azw3" bind:this={fileInput} onchange={handleImport} style="display: none;" />
      </label>
    </div>

    <!-- Library Location -->
    <div class="library-path-section">
      <div class="path-title">Emplacement</div>
      <div class="path-value" title={libraryPath}>{libraryPath || 'Aucun dossier sélectionné'}</div>
      <button class="change-path-btn" onclick={changeLibraryPath}>Changer le dossier</button>
    </div>

    <!-- Categories -->
    <nav class="nav-section">
      <div class="nav-title">Catégories</div>
      <ul class="nav-list">
        <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
        <li class="nav-item {selectedTag === null ? 'active' : ''}" role="button" onclick={() => filterByTag(null)}>
          <span>Tous les livres</span>
          <span class="nav-item-count">{totalBooksCount}</span>
        </li>
        {#each allTags as tag}
          <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
          <li class="nav-item nav-item-tag {selectedTag === tag ? 'active' : ''}" role="button" onclick={() => filterByTag(tag)}>
            <span># {tag}</span>
            <span class="nav-item-count">{tagCounts[tag] || 0}</span>
          </li>
        {/each}
      </ul>
    </nav>
  </aside>

  <!-- Main View -->
  <main class="main-content">
    <header class="header">
      <div class="search-bar">
        <span class="search-icon">🔍</span>
        <input 
          type="text" 
          placeholder="Rechercher par titre, auteur ou description..." 
          class="search-input"
          value={searchQuery}
          oninput={handleSearch}
        />
      </div>

      <div class="view-controls">
        <button class="view-btn {viewMode === 'grid' ? 'active' : ''}" onclick={() => viewMode = 'grid'}>
          <span>📱 Grille</span>
        </button>
        <button class="view-btn {viewMode === 'list' ? 'active' : ''}" onclick={() => viewMode = 'list'} style="margin-left: 6px;">
          <span>☰ Liste</span>
        </button>
      </div>
    </header>

    <!-- Books View -->
    <div class="books-container" bind:this={booksContainer} onscroll={handleScroll}>
      {#if books.length === 0}
        <div class="empty-placeholder" style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 300px; color: var(--text-muted);">
          <span style="font-size: 48px; margin-bottom: 12px;">📚</span>
          <p>Aucun livre trouvé dans la bibliothèque.</p>
        </div>
      {:else if viewMode === 'grid'}
        <div class="books-grid">
          {#each books as book}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="book-card {selectedBook && selectedBook.id === book.id ? 'selected' : ''}" onclick={() => selectedBook = book}>
              <div class="cover-wrapper">
                {#if book.coverPath}
                  <img src="/books/{book.id}/cover?t={Date.now()}" class="book-cover" alt="Couverture" />
                {:else}
                  <div class="no-cover-placeholder">
                    <span style="font-size: 40px;">📖</span>
                    <span style="font-size: 11px; font-weight: 500;">Pas de Couverture</span>
                  </div>
                {/if}
              </div>
              <div class="book-card-info">
                <div class="book-card-title" title={book.title}>{book.title}</div>
                <div class="book-card-author" title={book.author}>{book.author}</div>
                <div class="book-card-formats">
                  {#each book.formats as fmt}
                    <span class="format-badge">{fmt.format}</span>
                  {/each}
                </div>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="books-list">
          <table class="books-table">
            <thead>
              <tr>
                <th>Titre</th>
                <th>Auteur</th>
                <th>Éditeur</th>
                <th>Formats</th>
                <th>Ajouté le</th>
              </tr>
            </thead>
            <tbody>
              {#each books as book}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <tr class={selectedBook && selectedBook.id === book.id ? 'selected' : ''} onclick={() => selectedBook = book}>
                  <td><strong>{book.title}</strong></td>
                  <td>{book.author}</td>
                  <td>{book.publisher || '-'}</td>
                  <td>
                    {#each book.formats as fmt}
                      <span class="format-badge">{fmt.format}</span>
                    {/each}
                  </td>
                  <td>{new Date(book.addedAt).toLocaleDateString('fr-FR')}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  </main>

  <!-- Details Drawer -->
  <aside class="detail-drawer {selectedBook ? '' : 'hidden'}">
    {#if selectedBook}
      <div class="detail-header">
        <span class="nav-title">Détails du Livre</span>
        <button class="close-btn" onclick={() => selectedBook = null}>✕</button>
      </div>

      {#if selectedBook.coverPath}
        <img src="/books/{selectedBook.id}/cover?t={Date.now()}" class="detail-cover" alt="Couverture" />
      {:else}
        <div class="detail-cover" style="display: flex; flex-direction: column; align-items: center; justify-content: center; background: #1e1e28; color: var(--text-muted); gap: 12px;">
          <span style="font-size: 64px;">📖</span>
          <span>Pas de Couverture</span>
        </div>
      {/if}

      <h2 class="detail-title">{selectedBook.title}</h2>
      <div class="detail-author">par {selectedBook.author}</div>

      <div class="actions-row">
        <button class="btn-primary" onclick={() => readBook(selectedBook)}>Lire</button>
        <button class="btn-secondary" onclick={() => showEditDialog = true}>Éditer</button>
      </div>

      {#if isPluginRunning}
        <div style="color: var(--accent); font-size: 13px; font-weight: 500; margin-bottom: 12px; display: flex; align-items: center; gap: 8px; background: rgba(139, 92, 246, 0.1); padding: 8px; border-radius: 6px; border: 1px solid rgba(139, 92, 246, 0.2);">
          <span>⏳ Opération de plugin en cours...</span>
        </div>
      {/if}

      {#if plugins.length > 0}
        <div class="detail-section-title">Plugins & Transformations</div>
        <div style="display: flex; flex-direction: column; gap: 8px; margin-bottom: 20px;">
          {#each plugins as p}
            <button 
              class="btn-secondary" 
              onclick={() => applyPlugin(selectedBook.id, p.id)}
              style="text-align: left; font-size: 13px; padding: 10px; display: flex; flex-direction: column; align-items: flex-start; gap: 4px; width: 100%;"
              disabled={isPluginRunning}
            >
              <strong style="color: var(--accent);">{p.name}</strong>
              <span style="font-size: 11px; color: var(--text-muted); text-align: left;">{p.description}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="detail-section-title">Informations</div>
      {#if selectedBook.publisher}
        <div class="detail-text"><strong>Éditeur:</strong> {selectedBook.publisher}</div>
      {/if}
      {#if selectedBook.publishDate}
        <div class="detail-text"><strong>Date de pub.:</strong> {selectedBook.publishDate}</div>
      {/if}
      <div class="detail-text"><strong>Format:</strong> {selectedBook.formats[0] ? selectedBook.formats[0].format.toUpperCase() : '-'}</div>
      <div class="detail-text"><strong>Taille:</strong> {selectedBook.formats[0] ? (selectedBook.formats[0].filesize / 1024 / 1024).toFixed(2) : '0'} Mo</div>

      {#if selectedBook.tags && selectedBook.tags.length > 0}
        <div class="detail-section-title">Tags</div>
        <div class="tag-list">
          {#each selectedBook.tags as t}
            <span class="tag-badge">{t}</span>
          {/each}
        </div>
      {/if}

      {#if selectedBook.description}
        <div class="detail-section-title">Description</div>
        <div class="detail-text" style="font-size: 13px;">{selectedBook.description}</div>
      {/if}
      
      <button class="btn-secondary" onclick={() => handleDeleteBook(selectedBook.id)} style="border-color: var(--danger); color: var(--danger); width: 100%; margin-top: 24px;"> Supprimer le livre</button>
    {/if}
  </aside>
</div>

<!-- EPUB Reader Screen Overlay -->
{#if readingBook}
  <div class="reader-overlay" id="reader-overlay">
    <header class="reader-header">
      <h3 style="font-weight: 500;">{readingBook.title}</h3>
      <button class="btn-secondary" style="padding: 4px 12px;" onclick={closeReader}>Fermer la lecture</button>
    </header>
    
    <div class="reader-viewer">
      <button class="reader-nav-btn prev" onclick={prevPage}>◀</button>
      <div id="reader-area"></div>
      <button class="reader-nav-btn next" onclick={nextPage}>▶</button>
    </div>
  </div>
{/if}

<!-- Dialogs -->
<ConfirmDialog 
  show={showConfirmDelete} 
  title="Supprimer le livre"
  message="Êtes-vous sûr de vouloir supprimer ce livre définitivement ? Cette action est irréversible."
  confirmText="Supprimer"
  cancelText="Annuler"
  onConfirm={() => { showConfirmDelete = false; confirmDeleteResolve(true); }}
  onCancel={() => { showConfirmDelete = false; confirmDeleteResolve(false); }}
/>

<EditDialog 
  show={showEditDialog} 
  book={selectedBook}
  onclose={() => showEditDialog = false}
  onsave={handleSaveMetadata}
/>
