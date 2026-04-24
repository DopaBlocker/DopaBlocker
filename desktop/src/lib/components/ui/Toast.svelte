<!--
  Container de toasts. Renderizado uma vez no +layout.svelte; observa o
  store e empilha os toasts ativos no canto inferior direito. Cada toast
  tem cor de borda pelo tipo e botão de dismiss manual.
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { toast, type Toast } from '$lib/stores/toast';

    let toasts: Toast[] = $state([]);

    onMount(() => toast.subscribe((list) => (toasts = list)));
</script>

<div class="fixed bottom-4 right-4 z-[100] flex flex-col gap-2">
    {#each toasts as t (t.id)}
        <div
            class="card flex min-w-[280px] max-w-md items-start gap-3 px-4 py-3 text-sm shadow-lg animate-[slide-in_200ms_ease-out]"
            class:border-success={t.kind === 'success'}
            class:border-danger={t.kind === 'error'}
            class:border-border={t.kind === 'info'}
            role="status"
        >
            <span class="flex-1 leading-snug text-text">{t.message}</span>
            <button
                type="button"
                onclick={() => toast.dismiss(t.id)}
                aria-label="Dispensar"
                class="btn-icon shrink-0"
            >
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"
                    class="h-3.5 w-3.5">
                    <path d="M4 4l8 8M12 4l-8 8" stroke-linecap="round" />
                </svg>
            </button>
        </div>
    {/each}
</div>

<style>
    @keyframes slide-in {
        from {
            opacity: 0;
            transform: translateX(12px);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }
</style>
