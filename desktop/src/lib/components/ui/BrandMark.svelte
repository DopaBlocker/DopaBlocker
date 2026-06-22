<script lang="ts">
    // Marca do DopaBlocker — quadrado com gradiente azul→roxo e quadrado branco
    // interno. Centraliza o logo (antes duplicado em várias telas) e usa o
    // gradiente de marca dos tokens. Tamanhos: xs (sidebar), sm (login),
    // md (welcome), lg (hero).
    let { size = 'md' }: { size?: 'xs' | 'sm' | 'md' | 'lg' } = $props();

    const outer = { xs: 'h-7 w-7', sm: 'h-10 w-10', md: 'h-12 w-12', lg: 'h-16 w-16' };
    const inner = { xs: 'h-3 w-3', sm: 'h-4 w-4', md: 'h-5 w-5', lg: 'h-7 w-7' };
</script>

<div
    class="flex items-center justify-center rounded-lg shadow-(--shadow-card) {outer[size]}"
    style="background: linear-gradient(135deg, var(--brand-from), var(--brand-to))"
>
    <div class="rounded-sm bg-white/90 {inner[size]}"></div>
</div>
