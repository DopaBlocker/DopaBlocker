<!--
  Seletor de modo da conta. Três cards — Pessoal (avança o cadastro),
  Pais e Filhos (badge "Em breve", não avançam). Design intencional: mantém
  a UI final coerente sem implementar o fluxo parental no v0.1.
-->
<script lang="ts">
    type Mode = 'personal' | 'parent' | 'child';

    interface Props {
        selected?: Mode | null;
        onselect: (mode: Mode) => void;
    }

    let { selected = null, onselect }: Props = $props();

    const options: {
        mode: Mode;
        label: string;
        description: string;
        available: boolean;
    }[] = [
        {
            mode: 'personal',
            label: 'Pessoal',
            description: 'Bloqueie você mesmo. Uma única conta, uso individual.',
            available: true,
        },
        {
            mode: 'parent',
            label: 'Pais',
            description: 'Gerencie o bloqueio dos dispositivos dos seus filhos.',
            available: false,
        },
        {
            mode: 'child',
            label: 'Filhos',
            description: 'Vincule este dispositivo à conta de um responsável.',
            available: false,
        },
    ];
</script>

<div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
    {#each options as option (option.mode)}
        {@const active = selected === option.mode}
        <button
            type="button"
            onclick={() => onselect(option.mode)}
            class="group relative flex h-full flex-col items-start gap-2 rounded-md border p-4 text-left transition-colors"
            class:border-primary={active}
            class:bg-surface-2={active}
            class:border-border={!active}
            class:hover:border-border-strong={!active}
            class:bg-surface={!active}
        >
            <div class="flex w-full items-center justify-between">
                <span class="text-sm font-medium text-text">{option.label}</span>
                {#if !option.available}
                    <span class="badge-secondary">Em breve</span>
                {/if}
            </div>
            <p class="text-xs leading-relaxed text-text-muted">
                {option.description}
            </p>
        </button>
    {/each}
</div>
