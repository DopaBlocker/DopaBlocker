<!--
  Modal de exclusão de conta com dupla confirmação.

  Fluxo:
    1. step="confirm"     — pergunta "Tem certeza?"; se sim, vai para "type".
    2. step="type"        — pede o usuário digitar "EXCLUIR" para habilitar
                            o botão final.
    3. step="reauth"      — exibido se o Firebase exigir reautenticação
                            (`auth/requires-recent-login`). Botão desloga e
                            joga em /welcome para o user logar de novo.

  O componente não executa a exclusão em si — recebe um `onconfirm` que o
  caller (settings/+page.svelte) usa para orquestrar Firebase + backend +
  logout.
-->
<script lang="ts">
    import Modal from './ui/Modal.svelte';

    interface Props {
        open: boolean;
        onclose: () => void;
        /** Disparado quando o usuário digita EXCLUIR e clica no botão final.
         *  Espera-se que rejeite com `code === 'auth/requires-recent-login'`
         *  quando o Firebase exigir relogin. */
        onconfirm: () => Promise<void>;
        /** Disparado quando o usuário clica em "Fazer login de novo" no
         *  step de reauth. O caller deve fazer logout + navegar pra /welcome. */
        onreauth: () => void;
    }

    let { open, onclose, onconfirm, onreauth }: Props = $props();

    type Step = 'confirm' | 'type' | 'reauth';
    let step: Step = $state('confirm');
    let typed = $state('');
    let submitting = $state(false);
    let error: string | null = $state(null);

    const REQUIRED_TEXT = 'EXCLUIR';
    const canSubmit = $derived(typed.trim() === REQUIRED_TEXT && !submitting);

    function reset() {
        step = 'confirm';
        typed = '';
        submitting = false;
        error = null;
    }

    function handleClose() {
        if (submitting) return;
        reset();
        onclose();
    }

    async function handleConfirm() {
        if (!canSubmit) return;
        submitting = true;
        error = null;
        try {
            await onconfirm();
            reset();
            // Caller redireciona — não precisa fechar manualmente.
        } catch (err) {
            const code = (err as { code?: string }).code;
            if (code === 'auth/requires-recent-login') {
                step = 'reauth';
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            submitting = false;
        }
    }

    function handleReauth() {
        reset();
        onreauth();
    }
</script>

{#if step === 'confirm'}
    <Modal
        {open}
        title="Excluir conta?"
        description="Esta ação remove tudo permanentemente: sua conta, todos os bloqueios e qualquer filho vinculado. Não dá para desfazer."
        onclose={handleClose}
    >
        <div class="flex justify-end gap-2">
            <button type="button" class="btn-ghost" onclick={handleClose}>
                Cancelar
            </button>
            <button
                type="button"
                class="btn-primary bg-danger hover:bg-danger/90"
                onclick={() => (step = 'type')}
            >
                Continuar
            </button>
        </div>
    </Modal>
{:else if step === 'type'}
    <Modal
        {open}
        title="Confirme digitando"
        description={`Digite ${REQUIRED_TEXT} abaixo para confirmar a exclusão.`}
        onclose={handleClose}
    >
        <div class="flex flex-col gap-4">
            <input
                type="text"
                bind:value={typed}
                class="input"
                placeholder={REQUIRED_TEXT}
                autocomplete="off"
                spellcheck="false"
                disabled={submitting}
            />
            {#if error}
                <p class="text-xs text-danger">{error}</p>
            {/if}
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    class="btn-ghost"
                    onclick={handleClose}
                    disabled={submitting}
                >
                    Cancelar
                </button>
                <button
                    type="button"
                    class="btn-primary bg-danger hover:bg-danger/90 disabled:opacity-50"
                    onclick={handleConfirm}
                    disabled={!canSubmit}
                >
                    {submitting ? 'Excluindo…' : 'Excluir conta'}
                </button>
            </div>
        </div>
    </Modal>
{:else}
    <Modal
        {open}
        title="Sessão antiga"
        description="Por segurança, o Firebase exige que você tenha entrado recentemente para excluir a conta. Faça login de novo e tente outra vez."
        onclose={handleClose}
    >
        <div class="flex justify-end gap-2">
            <button type="button" class="btn-ghost" onclick={handleClose}>
                Cancelar
            </button>
            <button type="button" class="btn-primary" onclick={handleReauth}>
                Fazer login de novo
            </button>
        </div>
    </Modal>
{/if}
