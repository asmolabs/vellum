<script>
  export let show = false;
  export let title = "Confirmer l'action";
  export let message = "Êtes-vous sûr ?";
  export let confirmText = "Confirmer";
  export let cancelText = "Annuler";
  export let onConfirm = () => {};
  export let onCancel = () => {};
</script>

{#if show}
  <div class="dialog-overlay">
    <div class="dialog-box" style="width: 400px;">
      <div class="dialog-title" style="color: var(--danger);">{title}</div>
      <p style="margin-bottom: 24px; line-height: 1.5; color: var(--text-normal);">{message}</p>
      <div class="form-actions" style="justify-content: flex-end; gap: 12px; display: flex;">
        <button class="btn-secondary" on:click={onCancel}>{cancelText}</button>
        <button class="btn-primary" on:click={onConfirm} style="background-color: var(--danger); border-color: var(--danger);">{confirmText}</button>
      </div>
    </div>
  </div>
{/if}
