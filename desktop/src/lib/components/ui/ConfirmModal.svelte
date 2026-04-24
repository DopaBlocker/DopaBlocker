<!--
  Diálogo genérico de sim/não. Usa o Modal shell.
-->
<script lang="ts">
    import Modal from './Modal.svelte';

    interface Props {
        open: boolean;
        title: string;
        message: string;
        confirmLabel?: string;
        cancelLabel?: string;
        danger?: boolean;
        onconfirm: () => void;
        oncancel: () => void;
    }

    let {
        open,
        title,
        message,
        confirmLabel = 'Confirmar',
        cancelLabel = 'Cancelar',
        danger = false,
        onconfirm,
        oncancel,
    }: Props = $props();
</script>

<Modal {open} {title} onclose={oncancel}>
    <p class="text-sm leading-relaxed text-text-muted">{message}</p>

    <div class="mt-5 flex items-center justify-end gap-2">
        <button type="button" onclick={oncancel} class="btn-ghost">{cancelLabel}</button>
        {#if danger}
            <button type="button" onclick={onconfirm} class="btn-danger">{confirmLabel}</button>
        {:else}
            <button type="button" onclick={onconfirm} class="btn-primary">{confirmLabel}</button>
        {/if}
    </div>
</Modal>
