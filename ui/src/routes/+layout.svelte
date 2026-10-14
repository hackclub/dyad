<script lang="ts">
    import "$lib/palette.css";
    import favicon from "$lib/assets/favicon.svg";

    let { children } = $props();
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
    <script
        defer
        data-domain="dyad.hackclub.com"
        src="https://plausible.io/js/script.js"
    ></script>
</svelte:head>

{@render children()}
