<!-- Layout raiz — importa Tailwind CSS global e renderiza páginas filhas. -->
<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

{@render children()}
