<!--
  Tela de login: centralizada, full-bleed, sem sidebar.
  O redirect pós-login é tratado pelo +layout.svelte.
-->
<script lang="ts">
    import LoginForm from '$lib/components/LoginForm.svelte';
</script>

<div class="flex min-h-screen items-center justify-center bg-bg px-6">
    <LoginForm />
</div>
