<!--
  Tela do modo Pais. So acessivel quando auth.user.mode === 'parental'
  (gating no +layout.svelte). Filhos sao redirecionados para `/`.

  Conteudo: gerar codigo de vinculacao + listar/desvincular filhos.
  Adicionar/remover bloqueios e gerenciado em /blocking — a blocklist e
  unica e propaga para todos os filhos.
-->
<script lang="ts">
    import DeviceLinkCode from '$lib/components/DeviceLinkCode.svelte';
    import ParentalDashboard from '$lib/components/ParentalDashboard.svelte';
</script>

<div class="flex flex-col gap-6">
    <header>
        <div class="field-label">Modo Pais</div>
        <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">
            Filhos vinculados
        </h1>
        <p class="mt-1 text-sm text-text-muted">
            A blocklist da sua conta vale para todos os dispositivos filhos
            vinculados — voce mesmo (este dispositivo) nao recebe os bloqueios.
        </p>
    </header>

    <DeviceLinkCode />
    <ParentalDashboard />
</div>
