<script lang="ts">
    import type { Snippet } from 'svelte';

    // Estado vazio padrão: ícone + título + descrição + ação opcional.
    // Ícone e ação são snippets (padrão Svelte 5 do projeto).
    let {
        title,
        description = '',
        icon,
        action,
    }: {
        title: string;
        description?: string;
        icon?: Snippet;
        action?: Snippet;
    } = $props();
</script>

<div class="flex flex-col items-center justify-center gap-3 py-10 text-center">
    <div
        class="flex h-12 w-12 items-center justify-center rounded-lg border border-border bg-surface-2 text-text-muted"
    >
        {@render icon?.()}
    </div>
    <div class="flex flex-col gap-1">
        <h3 class="text-sm font-semibold text-text">{title}</h3>
        {#if description}
            <p class="max-w-xs text-xs leading-relaxed text-text-muted">{description}</p>
        {/if}
    </div>
    {@render action?.()}
</div>
