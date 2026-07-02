let books = [];
let libraryPath = '';
let searchQuery = '';
let selectedBook = null;
let selectedTag = null;
let allTags = [];
let viewMode = 'grid';
let plugins = [];
let isPluginRunning = false;

// Pagination state
let skip = 0;
const limit = 20;
let hasMore = true;
let isLoading = false;
let totalBooksCount = 0;
let tagCounts = {};

// EPUB Reader state
let readingBook = null;
let epubBook = null;
let rendition = null;

// DOM Elements
const booksGridView = document.getElementById('books-grid-view');
const booksListView = document.getElementById('books-list-view');
const booksTableBody = document.getElementById('books-table-body');
const noBooksPlaceholder = document.getElementById('no-books-placeholder');
const searchInput = document.getElementById('search-input');
const viewModeGridBtn = document.getElementById('view-mode-grid');
const viewModeListBtn = document.getElementById('view-mode-list');
const libraryPathText = document.getElementById('library-path-text');
const changeLibraryPathBtn = document.getElementById('change-library-path-btn');
const categoriesList = document.getElementById('categories-list');
const allBooksCount = document.getElementById('all-books-count');
const bookImportInput = document.getElementById('book-import-input');
const detailDrawer = document.getElementById('detail-drawer');
const detailDrawerContent = document.getElementById('detail-drawer-content');

// Edit Modal Elements
const editDialog = document.getElementById('edit-dialog');
const editBookId = document.getElementById('edit-book-id');
const editTitle = document.getElementById('edit-title');
const editAuthor = document.getElementById('edit-author');
const editPublisher = document.getElementById('edit-publisher');
const editDescription = document.getElementById('edit-description');
const editTags = document.getElementById('edit-tags');
const editCoverFile = document.getElementById('edit-cover-file');
const editCancelBtn = document.getElementById('edit-cancel-btn');
const editSaveBtn = document.getElementById('edit-save-btn');

// Reader Elements
const readerOverlay = document.getElementById('reader-overlay');
const readerTitle = document.getElementById('reader-title');
const readerCloseBtn = document.getElementById('reader-close-btn');
const readerPrevBtn = document.getElementById('reader-prev-btn');
const readerNextBtn = document.getElementById('reader-next-btn');

// Initialize App
window.addEventListener('DOMContentLoaded', () => {
  loadLibraryPath();
  loadTags();
  loadBooks(true);
  loadPlugins();
  setupEventListeners();
});

function setupEventListeners() {
  // Search with debounce
  let searchTimeout;
  searchInput.addEventListener('input', (e) => {
    searchQuery = e.target.value;
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      loadBooks(true);
    }, 300);
  });

  // View Mode
  viewModeGridBtn.addEventListener('click', () => {
    viewMode = 'grid';
    viewModeGridBtn.classList.add('active');
    viewModeListBtn.classList.remove('active');
    renderBooks();
  });

  viewModeListBtn.addEventListener('click', () => {
    viewMode = 'list';
    viewModeListBtn.classList.add('active');
    viewModeGridBtn.classList.remove('active');
    renderBooks();
  });

  // Infinite Scroll on books-container
  const booksContainer = document.querySelector('.books-container');
  booksContainer.addEventListener('scroll', () => {
    if (booksContainer.scrollHeight - booksContainer.scrollTop - booksContainer.clientHeight < 100) {
      if (hasMore && !isLoading) {
        loadBooks(false);
      }
    }
  });

  // Library Path
  changeLibraryPathBtn.addEventListener('click', async () => {
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
          libraryPathText.innerText = libraryPath;
          libraryPathText.title = libraryPath;
          selectedBook = null;
          detailDrawer.classList.add('hidden');
          await loadTags();
          loadBooks(true);
        }
      } catch (err) {
        alert("Erreur lors de la sélection du dossier: " + err);
      }
    } else {
      // Fallback
      const path = prompt("Entrez le chemin absolu du dossier de bibliothèque :", libraryPath);
      if (path) {
        const res = await fetch('/books/library-path', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path })
        });
        const data = await res.json();
        libraryPath = data.path;
        libraryPathText.innerText = libraryPath;
        libraryPathText.title = libraryPath;
        selectedBook = null;
        detailDrawer.classList.add('hidden');
        await loadTags();
        loadBooks(true);
      }
    }
  });

  // Book Import
  bookImportInput.addEventListener('change', async (e) => {
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
      selectBook(newBook);
    } catch (err) {
      alert("Erreur d'importation : " + err.message);
    } finally {
      bookImportInput.value = '';
    }
  });

  // Category All
  document.getElementById('category-all').addEventListener('click', () => {
    filterByTag(null);
  });

  // Edit Modal Cancel
  editCancelBtn.addEventListener('click', () => {
    editDialog.classList.add('hidden');
  });

  // Edit Modal Save
  editSaveBtn.addEventListener('click', saveMetadata);

  // Reader Close
  readerCloseBtn.addEventListener('click', closeReader);
  readerPrevBtn.addEventListener('click', prevPage);
  readerNextBtn.addEventListener('click', nextPage);
}

async function loadLibraryPath() {
  try {
    const res = await fetch('/books/library-path');
    const data = await res.json();
    libraryPath = data.path;
    libraryPathText.innerText = libraryPath || 'Aucun dossier sélectionné';
    libraryPathText.title = libraryPath || '';
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
    renderCategories();
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
    booksGridView.innerHTML = '';
    booksTableBody.innerHTML = '';
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
    
    books = books.concat(newBooks);
    skip = books.length;
    
    renderBooks();
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

function renderCategories() {
  // Update All count
  allBooksCount.innerText = totalBooksCount;

  // Clear older tags items
  const tagItems = categoriesList.querySelectorAll('.nav-item-tag');
  tagItems.forEach(el => el.remove());

  // Render tags list
  allTags.forEach(tag => {
    const li = document.createElement('li');
    li.className = 'nav-item nav-item-tag' + (selectedTag === tag ? ' active' : '');
    
    const count = tagCounts[tag] || 0;
    
    li.innerHTML = `
      <span># ${tag}</span>
      <span class="nav-item-count">${count}</span>
    `;
    
    li.addEventListener('click', () => {
      filterByTag(tag);
    });
    
    categoriesList.appendChild(li);
  });
}

function filterByTag(tag) {
  selectedTag = tag;
  
  // Update active class in categories list
  const allItems = categoriesList.querySelectorAll('.nav-item');
  allItems.forEach(el => el.classList.remove('active'));
  
  if (tag === null) {
    document.getElementById('category-all').classList.add('active');
  }
  
  loadBooks(true);
}

function renderBooks() {
  // Clear lists
  booksGridView.innerHTML = '';
  booksTableBody.innerHTML = '';
  
  if (books.length === 0) {
    noBooksPlaceholder.classList.remove('hidden');
    booksGridView.classList.add('hidden');
    booksListView.classList.add('hidden');
    return;
  }
  
  noBooksPlaceholder.classList.add('hidden');
  
  if (viewMode === 'grid') {
    booksGridView.classList.remove('hidden');
    booksListView.classList.add('hidden');
    
    books.forEach(book => {
      const card = document.createElement('div');
      card.className = 'book-card' + (selectedBook && selectedBook.id === book.id ? ' selected' : '');
      
      const coverUrl = book.coverPath ? `/books/${book.id}/cover?t=${Date.now()}` : null;
      
      card.innerHTML = `
        <div class="cover-wrapper">
          ${coverUrl ? `<img src="${coverUrl}" class="book-cover" alt="Couverture" />` : `
            <div class="no-cover-placeholder">
              <span style="font-size: 40px;">📖</span>
              <span style="font-size: 11px; font-weight: 500;">Pas de Couverture</span>
            </div>
          `}
        </div>
        <div class="book-card-info">
          <div class="book-card-title" title="${book.title}">${book.title}</div>
          <div class="book-card-author" title="${book.author}">${book.author}</div>
          <div class="book-card-formats">
            ${book.formats.map(fmt => `<span class="format-badge">${fmt.format}</span>`).join('')}
          </div>
        </div>
      `;
      
      card.addEventListener('click', () => selectBook(book));
      booksGridView.appendChild(card);
    });
  } else {
    booksGridView.classList.add('hidden');
    booksListView.classList.remove('hidden');
    
    books.forEach(book => {
      const tr = document.createElement('tr');
      tr.className = (selectedBook && selectedBook.id === book.id ? 'selected' : '');
      
      const formattedDate = new Date(book.addedAt).toLocaleDateString('fr-FR');
      
      tr.innerHTML = `
        <td><strong>${book.title}</strong></td>
        <td>${book.author}</td>
        <td>${book.publisher || '-'}</td>
        <td>
          ${book.formats.map(fmt => `<span class="format-badge">${fmt.format}</span>`).join('')}
        </td>
        <td>${formattedDate}</td>
      `;
      
      tr.addEventListener('click', () => selectBook(book));
      booksTableBody.appendChild(tr);
    });
  }
}

function selectBook(book) {
  selectedBook = book;
  
  // Re-render to highlight selection
  renderBooks();
  
  if (!book) {
    detailDrawer.classList.add('hidden');
    return;
  }
  
  detailDrawer.classList.remove('hidden');
  
  const coverUrl = book.coverPath ? `/books/${book.id}/cover?t=${Date.now()}` : null;
  const mainFormat = book.formats[0];
  const mainFormatText = mainFormat ? mainFormat.format.toUpperCase() : '-';
  const sizeMb = mainFormat ? (mainFormat.filesize / 1024 / 1024).toFixed(2) : '0';
  
  detailDrawerContent.innerHTML = `
    <div class="detail-header">
      <span class="nav-title">Détails du Livre</span>
      <button class="close-btn" id="detail-close-btn">✕</button>
    </div>

    ${coverUrl ? `<img src="${coverUrl}" class="detail-cover" alt="Couverture" />` : `
      <div class="detail-cover" style="display: flex; flex-direction: column; align-items: center; justify-content: center; background: #1e1e28; color: var(--text-muted); gap: 12px;">
        <span style="font-size: 64px;">📖</span>
        <span>Pas de Couverture</span>
      </div>
    `}

    <h2 class="detail-title">${book.title}</h2>
    <div class="detail-author">par ${book.author}</div>

    <div class="actions-row">
      <button class="btn-primary" id="detail-read-btn">Lire</button>
      <button class="btn-secondary" id="detail-edit-btn">Éditer</button>
    </div>

    ${isPluginRunning ? `
      <div style="color: var(--accent); font-size: 13px; font-weight: 500; margin-bottom: 12px; display: flex; align-items: center; gap: 8px; background: rgba(139, 92, 246, 0.1); padding: 8px; border-radius: 6px; border: 1px solid rgba(139, 92, 246, 0.2);">
        <span>⏳ Opération de plugin en cours...</span>
      </div>
    ` : ''}

    ${plugins.length > 0 ? `
      <div class="detail-section-title">Plugins & Transformations</div>
      <div style="display: flex; flex-direction: column; gap: 8px; margin-bottom: 20px;">
        ${plugins.map(p => `
          <button 
            class="btn-secondary plugin-run-btn" 
            data-plugin-id="${p.id}"
            style="text-align: left; font-size: 13px; padding: 10px; display: flex; flex-direction: column; align-items: flex-start; gap: 4px; width: 100%;"
            ${isPluginRunning ? 'disabled' : ''}
          >
            <strong style="color: var(--accent);">${p.name}</strong>
            <span style="font-size: 11px; color: var(--text-muted); text-align: left;">${p.description}</span>
          </button>
        `).join('')}
      </div>
    ` : ''}

    <div class="detail-section-title">Informations</div>
    ${book.publisher ? `<div class="detail-text"><strong>Éditeur:</strong> ${book.publisher}</div>` : ''}
    ${book.publishDate ? `<div class="detail-text"><strong>Date de pub.:</strong> ${book.publishDate}</div>` : ''}
    <div class="detail-text"><strong>Format:</strong> ${mainFormatText}</div>
    <div class="detail-text"><strong>Taille:</strong> ${sizeMb} Mo</div>

    ${book.tags && book.tags.length > 0 ? `
      <div class="detail-section-title">Tags</div>
      <div class="tag-list">
        ${book.tags.map(t => `<span class="tag-badge">${t}</span>`).join('')}
      </div>
    ` : ''}

    ${book.description ? `
      <div class="detail-section-title">Description</div>
      <div class="detail-text" style="font-size: 13px;">${book.description}</div>
    ` : ''}
    
    <button class="btn-secondary" id="detail-delete-btn" style="border-color: var(--danger); color: var(--danger); width: 100%; margin-top: 24px;"> Supprimer le livre</button>
  `;
  
  // Attach Detail Event Listeners
  document.getElementById('detail-close-btn').addEventListener('click', () => selectBook(null));
  document.getElementById('detail-read-btn').addEventListener('click', () => readBook(book));
  document.getElementById('detail-edit-btn').addEventListener('click', openEditDialog);
  document.getElementById('detail-delete-btn').addEventListener('click', () => deleteBook(book.id));
  
  const pluginButtons = detailDrawerContent.querySelectorAll('.plugin-run-btn');
  pluginButtons.forEach(btn => {
    btn.addEventListener('click', (e) => {
      const pId = btn.getAttribute('data-plugin-id');
      applyPlugin(book.id, pId);
    });
  });
}

async function applyPlugin(bookId, pluginId) {
  isPluginRunning = true;
  selectBook(selectedBook); // Re-render details to show loading state
  
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
    // Select the new book version
    const updatedSelectedBook = books.find(b => b.id === newBook.id) || newBook;
    selectBook(updatedSelectedBook);
  } catch (err) {
    isPluginRunning = false;
    selectBook(selectedBook);
    alert("Échec de l'exécution du plugin : " + err.message);
  }
}

function openEditDialog() {
  if (!selectedBook) return;
  editBookId.value = selectedBook.id;
  editTitle.value = selectedBook.title;
  editAuthor.value = selectedBook.author;
  editPublisher.value = selectedBook.publisher || '';
  editDescription.value = selectedBook.description || '';
  editTags.value = selectedBook.tags ? selectedBook.tags.join(', ') : '';
  editCoverFile.value = '';
  
  editDialog.classList.remove('hidden');
}

async function saveMetadata() {
  const id = editBookId.value;
  const tags = editTags.value
    .split(',')
    .map(t => t.trim())
    .filter(t => t.length > 0);

  const payload = {
    title: editTitle.value,
    author: editAuthor.value,
    publisher: editPublisher.value,
    description: editDescription.value,
    tags: tags
  };

  try {
    let res = await fetch(`/books/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    if (!res.ok) throw new Error(await res.text());
    let updatedBook = await res.json();

    const coverFile = editCoverFile.files[0];
    if (coverFile) {
      const formData = new FormData();
      formData.append('cover', coverFile);
      res = await fetch(`/books/${id}/cover`, {
        method: 'POST',
        body: formData
      });
      if (!res.ok) throw new Error(await res.text());
      updatedBook = await res.json();
    }

    editDialog.classList.add('hidden');
    await loadTags();
    await loadBooks(true);
    // Refresh selected book details
    const refreshed = books.find(b => b.id === updatedBook.id) || updatedBook;
    selectBook(refreshed);
  } catch (err) {
    alert("Échec de la mise à jour : " + err.message);
  }
}

async function deleteBook(id) {
  if (!confirm('Êtes-vous sûr de vouloir supprimer ce livre définitivement ?')) return;
  
  try {
    const res = await fetch(`/books/${id}`, {
      method: 'DELETE'
    });
    if (!res.ok) throw new Error(await res.text());
    selectedBook = null;
    detailDrawer.classList.add('hidden');
    await loadTags();
    await loadBooks(true);
  } catch (err) {
    alert("Échec de la suppression: " + err.message);
  }
}

// EPUB Reader
async function readBook(book) {
  const epubFormat = book.formats.find(f => f.format === 'epub');
  if (!epubFormat) {
    alert("La liseuse intégrée ne prend en charge que le format EPUB pour l'instant.");
    return;
  }

  readingBook = book;
  readerTitle.innerText = book.title;
  readerOverlay.classList.remove('hidden');

  try {
    const res = await fetch(`/books/${book.id}/file/epub`);
    const buffer = await res.arrayBuffer();

    if (epubBook) {
      epubBook.destroy();
    }
    document.getElementById('reader-area').innerHTML = '';

    epubBook = ePub(buffer);
    rendition = epubBook.renderTo('reader-area', {
      width: '100%',
      height: '100%',
      flow: 'paginated'
    });

    await rendition.display();
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
  readerOverlay.classList.add('hidden');
}

function prevPage() {
  if (rendition) rendition.prev();
}

function nextPage() {
  if (rendition) rendition.next();
}
