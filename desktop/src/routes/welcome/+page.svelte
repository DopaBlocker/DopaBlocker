<!--
  Tela de boas-vindas — primeira coisa que o usuário vê quando abre o app sem
  sessão. Três cards full-screen:
    - Pessoal → /login?mode=personal
    - Pais    → /login?mode=parental
    - Filhos  → /onboarding/child   (sem login)

  Sem ModeSelector embutido em formulário de signup; aqui a escolha vem ANTES
  de qualquer credencial. Lógica de auth vive em /login (Pessoal/Pais) ou
  /onboarding/child (Filhos).
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import BrandMark from '$lib/components/ui/BrandMark.svelte';

    type Mode = 'personal' | 'parental' | 'child';

    interface Option {
        mode: Mode;
        label: string;
        description: string;
        icon: 'user' | 'parental' | 'child';
    }

    const options: Option[] = [
        {
            mode: 'personal',
            label: 'Pessoal',
            description:
                'Bloqueie seus próprios sites e apps. Uma única conta, uso individual.',
            icon: 'user',
        },
        {
            mode: 'parental',
            label: 'Pais',
            description:
                'Gere um código de vinculação e controle os bloqueios dos dispositivos dos seus filhos.',
            icon: 'parental',
        },
        {
            mode: 'child',
            label: 'Filhos',
            description:
                'Vincule este dispositivo a uma conta de responsável digitando o código de 6 dígitos.',
            icon: 'child',
        },
    ];

    function handleSelect(mode: Mode) {
        if (mode === 'child') {
            void goto('/onboarding/child');
        } else {
            void goto(`/login?mode=${mode}`);
        }
    }
</script>

<div class="relative flex min-h-screen flex-col items-center justify-center overflow-hidden bg-bg p-6">
    <!-- Glow de marca atrás do header. -->
    <div
        class="pointer-events-none absolute left-1/2 top-20 h-72 w-72 -translate-x-1/2 rounded-full opacity-10 blur-3xl"
        style="background: linear-gradient(135deg, var(--brand-from), var(--brand-to))"
    ></div>
    <div class="relative w-full max-w-3xl">
        <!-- Header -->
        <div class="mb-10 flex flex-col items-center gap-3 text-center">
            <BrandMark size="md" />
            <div>
                <h1 class="text-2xl font-semibold tracking-tight text-gradient">
                    DopaBlocker
                </h1>
                <p class="mt-2 text-sm text-text-muted">
                    Como você vai usar o app neste dispositivo?
                </p>
            </div>
        </div>

        <!-- 3 cards -->
        <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
            {#each options as option (option.mode)}
                <button
                    type="button"
                    onclick={() => handleSelect(option.mode)}
                    style="border-top-color: var(--color-hairline)"
                    class="group flex flex-col items-start gap-3 rounded-lg border border-border bg-surface p-5 text-left shadow-(--shadow-card) transition-all hover:border-primary hover:bg-surface-2 focus-visible:border-primary focus-visible:bg-surface-2"
                >
                    <div
                        class="flex h-10 w-10 items-center justify-center rounded-md border border-border bg-surface-2 text-text-muted transition-colors group-hover:border-primary group-hover:text-primary"
                    >
                        {#if option.icon === 'user'}
                            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                stroke-width="1.5" class="h-5 w-5">
                                <circle cx="8" cy="6" r="2.5" />
                                <path d="M3 13c0-2.5 2.5-4 5-4s5 1.5 5 4"
                                    stroke-linecap="round" />
                            </svg>
                        {:else if option.icon === 'parental'}
                            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                stroke-width="1.5" class="h-5 w-5">
                                <circle cx="5" cy="5" r="2" />
                                <circle cx="11" cy="5" r="2" />
                                <path d="M2 13c0-1.66 1.79-3 3.5-3 .83 0 1.59.31 2.16.82M14 13c0-1.66-1.79-3-3.5-3-.83 0-1.59.31-2.16.82"
                                    stroke-linecap="round" />
                            </svg>
                        {:else}
                            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                stroke-width="1.5" class="h-5 w-5">
                                <rect x="3" y="6" width="10" height="7" rx="1" />
                                <path d="M5 6V4a3 3 0 016 0v2"
                                    stroke-linecap="round" />
                            </svg>
                        {/if}
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <span class="text-base font-semibold text-text">{option.label}</span>
                        <p class="text-xs leading-relaxed text-text-muted">
                            {option.description}
                        </p>
                    </div>
                </button>
            {/each}
        </div>

        <p class="mt-8 text-center text-[11px] text-text-dim">
            Você pode alternar entre Pessoal e Pais quando quiser, nas Configurações.
        </p>
    </div>
</div>
