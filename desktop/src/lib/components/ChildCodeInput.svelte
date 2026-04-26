<!--
  Input de codigo de 6 digitos para o fluxo "Filhos".

  - 6 inputs separados, cada um aceita 1 digito.
  - Auto-advance: digitar avanca o foco; Backspace volta.
  - Suporta paste do codigo inteiro (`onpaste` distribui dígitos por todos os
    inputs).
  - Valida e dispara `onsubmit` automaticamente quando os 6 ficam preenchidos.

  Esse componente e UI pura — sem chamada ao backend nem ao auth store. Quem
  chama (rota `/onboarding/child`) recebe o codigo no callback e lida com a
  request, exibicao de erro, etc. Mantem o componente reutilizavel.
-->
<script lang="ts">
    interface Props {
        /** Disparado quando os 6 digitos sao preenchidos. */
        onsubmit: (code: string) => void;
        /** True desabilita os inputs (ex: durante a request). */
        disabled?: boolean;
        /** Mensagem de erro a ser exibida abaixo dos inputs. */
        error?: string | null;
    }

    let { onsubmit, disabled = false, error = null }: Props = $props();

    let digits: string[] = $state(['', '', '', '', '', '']);
    const inputs: (HTMLInputElement | null)[] = [null, null, null, null, null, null];

    function focusAt(index: number) {
        const target = inputs[index];
        if (target) target.focus();
    }

    function maybeSubmit() {
        if (digits.every((d) => d.length === 1)) {
            onsubmit(digits.join(''));
        }
    }

    function handleInput(index: number, e: Event) {
        const target = e.target as HTMLInputElement;
        const value = target.value.replace(/\D/g, '').slice(-1);
        digits[index] = value;
        target.value = value;

        if (value && index < 5) {
            focusAt(index + 1);
        }

        maybeSubmit();
    }

    function handleKeydown(index: number, e: KeyboardEvent) {
        if (e.key === 'Backspace' && !digits[index] && index > 0) {
            focusAt(index - 1);
        } else if (e.key === 'ArrowLeft' && index > 0) {
            e.preventDefault();
            focusAt(index - 1);
        } else if (e.key === 'ArrowRight' && index < 5) {
            e.preventDefault();
            focusAt(index + 1);
        }
    }

    function handlePaste(e: ClipboardEvent) {
        const pasted = (e.clipboardData?.getData('text') ?? '').replace(/\D/g, '').slice(0, 6);
        if (!pasted) return;
        e.preventDefault();
        for (let i = 0; i < 6; i++) {
            digits[i] = pasted[i] ?? '';
        }
        focusAt(Math.min(pasted.length, 5));
        maybeSubmit();
    }

    export function clear() {
        digits = ['', '', '', '', '', ''];
        focusAt(0);
    }
</script>

<div class="flex flex-col gap-3">
    <div class="flex justify-center gap-2" onpaste={handlePaste}>
        {#each digits as digit, i (i)}
            <input
                bind:this={inputs[i]}
                type="text"
                inputmode="numeric"
                autocomplete={i === 0 ? 'one-time-code' : 'off'}
                maxlength={1}
                pattern="[0-9]"
                value={digit}
                {disabled}
                oninput={(e) => handleInput(i, e)}
                onkeydown={(e) => handleKeydown(i, e)}
                aria-label={`Digito ${i + 1} do codigo`}
                class="h-12 w-10 rounded-md border border-border bg-surface text-center text-lg font-semibold text-text outline-none focus:border-primary focus:ring-1 focus:ring-primary disabled:opacity-50"
            />
        {/each}
    </div>

    {#if error}
        <p class="text-center text-xs text-danger">{error}</p>
    {/if}
</div>
