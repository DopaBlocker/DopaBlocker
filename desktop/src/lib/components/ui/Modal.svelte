<!--
  Shell de modal. Overlay escuro + dialog centralizado. Fecha em:
    - click no overlay
    - Esc
    - botão X no header (se `closable`)
  Conteúdo vai no slot `children`.
-->
<script lang="ts">
    interface Props {
        open: boolean;
        title: string;
        description?: string;
        closable?: boolean;
        onclose: () => void;
        children?: import('svelte').Snippet;
        footer?: import('svelte').Snippet;
    }

    let {
        open,
        title,
        description,
        closable = true,
        onclose,
        children,
        footer,
    }: Props = $props();

    function onOverlayClick(e: MouseEvent) {
        if (e.target === e.currentTarget && closable) onclose();
    }

    function onKey(e: KeyboardEvent) {
        if (e.key === 'Escape' && closable && open) onclose();
    }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
    <!-- Overlay clicável (close-on-click) — semanticamente `presentation`, o
         dialog real é o filho. Escape é tratado no window.onkeydown. -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        onclick={onOverlayClick}
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-4 backdrop-blur-sm"
    >
        <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="modal-title"
            tabindex="-1"
            class="card w-full max-w-md animate-[fade-in_150ms_ease-out]"
            style="box-shadow: var(--shadow-overlay)"
        >
            <header
                class="flex items-start justify-between gap-4 border-b border-border px-5 py-4"
            >
                <div>
                    <h2 id="modal-title" class="text-sm font-semibold text-text">
                        {title}
                    </h2>
                    {#if description}
                        <p class="mt-1 text-xs text-text-muted">{description}</p>
                    {/if}
                </div>
                {#if closable}
                    <button
                        type="button"
                        aria-label="Fechar"
                        onclick={onclose}
                        class="btn-icon"
                    >
                        <svg
                            viewBox="0 0 16 16"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            class="h-4 w-4"
                        >
                            <path d="M4 4l8 8M12 4l-8 8" stroke-linecap="round" />
                        </svg>
                    </button>
                {/if}
            </header>

            <div class="px-5 py-5">
                {@render children?.()}
            </div>

            {#if footer}
                <footer
                    class="flex items-center justify-end gap-2 border-t border-border px-5 py-3"
                >
                    {@render footer()}
                </footer>
            {/if}
        </div>
    </div>
{/if}

<style>
    @keyframes fade-in {
        from {
            opacity: 0;
            transform: translateY(4px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>
