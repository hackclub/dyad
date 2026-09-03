<script lang="ts">
    import "$lib/palette.css";
    import favicon from "$lib/assets/favicon.svg";

    let { children } = $props();
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<footer class="site-footer">
    <a
        href="https://hackclub.com/privacy-and-terms"
        style="color: #993100;">Hack Club privacy & Terms</a
    >
    <p style="margin-left: 3vw; margin-right: 3vw">
        Made by <a
            href="https://hackclub.enterprise.slack.com/team/U08LEHUPSCR"
            style="color: #993100;">Brendan</a
        > @ Hack Club HQ. By teens, for teens.
    </p>
    <a
        href="https://forms.hackclub.com/bounty"
        style="color: #993100;">Fulfillment Bounty</a
    >
</footer>

<style>
    .site-footer {
        height: 80px;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: var(--bg-primary);
        color: var(--text-dark);
    }
</style>
