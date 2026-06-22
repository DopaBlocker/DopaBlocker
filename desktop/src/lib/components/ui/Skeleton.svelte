<script lang="ts">
    // Placeholder de carregamento (shimmer). Use width/height via `class`.
    let { class: className = '' }: { class?: string } = $props();
</script>

<div class="skeleton {className}" aria-hidden="true"></div>
