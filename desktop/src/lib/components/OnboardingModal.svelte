<!--
  Modal mostrado no primeiro login. Explica as permissões que o app vai pedir
  (UAC, troca de DNS, filtros kernel) e o que fica armazenado onde. Fechamento
  persiste um flag em localStorage escopado ao user.id — se o mesmo PC tiver
  múltiplos usuários, cada um vê uma vez.
-->
<script lang="ts">
    import Modal from './ui/Modal.svelte';

    interface Props {
        open: boolean;
        onclose: () => void;
    }

    let { open, onclose }: Props = $props();
</script>

<Modal {open} title="Bem-vindo ao DopaBlocker" {onclose}>
    <div class="flex flex-col gap-4 text-sm leading-relaxed text-text">
        <p class="text-text-muted">
            Antes de você ativar o bloqueio pela primeira vez, o que acontece nos
            bastidores:
        </p>

        <div class="flex gap-3">
            <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-white">
                1
            </div>
            <div>
                <div class="font-medium">Pede permissão de administrador</div>
                <div class="text-xs text-text-muted">
                    Windows precisa disso pra trocar DNS do sistema e instalar os
                    filtros kernel que impedem bypass.
                </div>
            </div>
        </div>

        <div class="flex gap-3">
            <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-white">
                2
            </div>
            <div>
                <div class="font-medium">Bloqueia em duas camadas</div>
                <div class="text-xs text-text-muted">
                    Um resolvedor DNS local intercepta queries e um filtro
                    kernel-level (WFP) impede que o tráfego contorne por
                    DNS-over-HTTPS ou por IPs de resolvers conhecidos.
                </div>
            </div>
        </div>

        <div class="flex gap-3">
            <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-white">
                3
            </div>
            <div>
                <div class="font-medium">Seus dados ficam no seu disco</div>
                <div class="text-xs text-text-muted">
                    A lista de bloqueios é cacheada localmente em SQLCipher
                    (AES-256). A chave mora no Windows Credential Manager,
                    nunca em arquivo.
                </div>
            </div>
        </div>

        <div class="flex gap-3">
            <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-white">
                4
            </div>
            <div>
                <div class="font-medium">Firebase guarda só o essencial</div>
                <div class="text-xs text-text-muted">
                    Email e nome pra identificar sua conta. Nenhum histórico de
                    navegação nem das queries DNS sai do seu computador.
                </div>
            </div>
        </div>

        <button type="button" onclick={onclose} class="btn-primary mt-2 w-full">
            Entendi, vamos começar
        </button>
    </div>
</Modal>
