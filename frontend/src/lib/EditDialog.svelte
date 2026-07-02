<script>
  export let show = false;
  export let book = null;
  export let onclose = () => {};
  export let onsave = (data, file) => {};

  let title = '';
  let author = '';
  let publisher = '';
  let description = '';
  let tagsString = '';
  let coverFileInput = null;

  // React to book changes
  $: if (book) {
    title = book.title || '';
    author = book.author || '';
    publisher = book.publisher || '';
    description = book.description || '';
    tagsString = book.tags ? book.tags.join(', ') : '';
  }

  function handleCancel() {
    onclose();
  }

  function handleSave() {
    const tags = tagsString
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0);

    const updatedData = {
      title,
      author,
      publisher,
      description,
      tags
    };

    const file = coverFileInput && coverFileInput.files[0] ? coverFileInput.files[0] : null;
    onsave(updatedData, file);
  }
</script>

{#if show}
  <div class="dialog-overlay" id="edit-dialog">
    <div class="dialog-box">
      <div class="dialog-title">Éditer les Métadonnées</div>
      
      <div class="form-group">
        <label class="path-title">Titre</label>
        <input type="text" class="form-control" bind:value={title} />
      </div>

      <div class="form-group">
        <label class="path-title">Auteur</label>
        <input type="text" class="form-control" bind:value={author} />
      </div>

      <div class="form-group">
        <label class="path-title">Éditeur</label>
        <input type="text" class="form-control" bind:value={publisher} />
      </div>

      <div class="form-group">
        <label class="path-title">Description</label>
        <textarea class="form-control" rows="4" bind:value={description}></textarea>
      </div>

      <div class="form-group">
        <label class="path-title">Tags (séparés par des virgules)</label>
        <input type="text" class="form-control" bind:value={tagsString} />
      </div>

      <div class="form-group">
        <label class="path-title">Modifier la couverture (Image)</label>
        <input type="file" accept="image/*" bind:this={coverFileInput} />
      </div>

      <div class="form-actions">
        <button class="btn-secondary" onclick={handleCancel}>Annuler</button>
        <button class="btn-primary" onclick={handleSave}>Enregistrer</button>
      </div>
    </div>
  </div>
{/if}
