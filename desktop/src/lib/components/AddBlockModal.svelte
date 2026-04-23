<!--
  Modal para adicionar um item à blocklist. Tabs do tipo (Site/App/Palavra) +
  input validado. Normaliza URLs antes de submeter (remove protocolo/www/path).
-->
<script lang="ts">
    import type { BlockedType } from '$lib/types';
    import Modal from './ui/Modal.svelte';

    interface Props {
        open: boolean;
        onclose: () => void;
        onsubmit: (type: BlockedType, value: string) => Promise<void>;
    }

    let { open, onclose, onsubmit }: Props = $props();

    const TYPES: { id: BlockedType; label: string; placeholder: string; helper: string }[] = [
        {
            id: 'domain',
            label: 'Site',
            placeholder: 'instagram.com',
            helper: 'Bloqueia o domínio e todos os subdomínios.',
        },
        {
            id: 'app',
            label: 'App',
            placeholder: 'com.instagram.android',
            helper: 'Package name ou nome do executável.',
        },
        {
            id: 'keyword',
            label: 'Palavra-chave',
            placeholder: 'cassino',
            helper: 'Bloqueia buscas e domínios que contenham a palavra.',
        },
    ];

    let selected = $state<BlockedType>('domain');
    let value = $state('');
    let submitting = $state(false);
    let errorMsg = $state<string | null>(null);

    const currentType = $derived(TYPES.find((t) => t.id === selected)!);

    function reset() {
        selected = 'domain';
        value = '';
        submitting = false;
        errorMsg = null;
    }

    function close() {
        reset();
        onclose();
    }

    // Normaliza site ("https://www.X.com/path" → "x.com"). Para app/keyword,
    // só faz trim. Mantém simples — a normalização pesada fica no engine Rust.
    function normalize(input: string): string {
        let s = input.trim();
        if (selected !== 'domain') return s;
        s = s.toLowerCase();
        s = s.replace(/^https?:\/\//, '');
        s = s.replace(/^www\./, '');
        s = s.split('/')[0];
        s = s.split(':')[0];
        return s;
    }

    async function submit(e: Event) {
        e.preventDefault();
        if (submitting) return;
        errorMsg = null;

        const normalized = normalize(value);
        if (!normalized) {
            errorMsg = 'Preencha um valor válido.';
            return;
        }
        if (selected === 'domain' && !/\.[a-z]{2,}$/.test(normalized)) {
            errorMsg = 'Formato de domínio inválido.';
            return;
        }

        submitting = true;
        try {
            await onsubmit(selected, normalized);
            close();
        } catch (err) {
            errorMsg = err instanceof Error ? err.message : String(err);
            submitting = false;
        }
    }
</script>

<Modal {open} title="Adicionar bloqueio" onclose={close}>
    <form class="flex flex-col gap-4" onsubmit={submit}>
        <div class="flex gap-1 rounded-md border border-border bg-surface-2 p-1">
            {#each TYPES as t (t.id)}
                <button
                    type="button"
                    onclick={() => {
                        selected = t.id;
                        errorMsg = null;
                    }}
                    class="flex-1 rounded px-3 py-1.5 text-xs font-medium transition-colors"
                    class:bg-primary={selected === t.id}
                    class:text-white={selected === t.id}
                    class:text-text-muted={selected !== t.id}
                    class:hover:text-text={selected !== t.id}
                >
                    {t.label}
                </button>
            {/each}
        </div>

        <label class="flex flex-col gap-1.5">
            <span class="field-label">Valor</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input
                type="text"
                required
                autofocus
                bind:value
                placeholder={currentType.placeholder}
                class="input"
            />
            <span class="text-xs text-text-dim">{currentType.helper}</span>
        </label>

        {#if errorMsg}
            <div class="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-xs text-danger">
                {errorMsg}
            </div>
        {/if}

        <div class="mt-2 flex items-center justify-end gap-2">
            <button type="button" onclick={close} class="btn-ghost">Cancelar</button>
            <button type="submit" disabled={submitting} class="btn-primary">
                {submitting ? 'Adicionando…' : 'Adicionar'}
            </button>
        </div>
    </form>
</Modal>
