<script>
    import "$lib/palette.css";
    import BackgroundLines from "$lib/BackgroundLines.svelte";
</script>

<BackgroundLines />

<!-- Title -->
<div class="title">
    <h3 class="title-subtext">Hack Club</h3>
    <h1 class="title-text">DUOS</h1>
</div>

<!-- Orpheus -->
<a href="https://hackclub.com/"
    ><img
        style="position: absolute; top: 0; right: 2.5vw; border: 0; width: 25vw; z-index: 999;"
        src="https://assets.hackclub.com/flag-orpheus-top.svg"
        alt="Hack Club"
    /></a
>

<!-- Dashboard button -->
<a href="/login" class="dashboard-btn">Open Dashboard</a>

<!-- Little nub -->

<style>
    :root {
        overflow-x: hidden;
    }

    .title {
        height: 65vh;
        width: calc(70vw + 4px);
        padding-top: 5vh;
        text-align: center;
        display: flex;
        flex-direction: column;
        justify-content: center;
        container-type: size;
    }
    .title-subtext {
        font-size: 12cqmin;
        font-weight: light;
        text-box-trim: trim-both;
        line-height: 1;
        margin: 0;
        color: var(--accent-secondary);
        text-shadow:
            -1px 0 var(--border),
            0 1px var(--border),
            1px 0 var(--border),
            0 -1px var(--border);
    }
    .title-text {
        font-size: 40cqmin;
        text-box-edge: text;
        text-box-trim: trim-both;
        font-weight: lighter;
        line-height: 1;
        margin-top: -5cqmin;
        margin-bottom: 0;
        color: var(--accent-primary);
        text-shadow:
            -1px 0 var(--border),
            0 1px var(--border),
            1px 0 var(--border),
            0 -1px var(--border);
    }

    .dashboard-btn {
        color: var(--bg-secondary);
        text-decoration: none;
        background-color: var(--accent-primary);
        width: calc(30vw + 5vh - 3px);
        height: 10vh;
        font-size: 6vh;
        line-height: 10vh;
        display: block;
        text-align: center;
        border-top-right-radius: 3vh;
        border-bottom-right-radius: 5vh;
        position: absolute;
        margin-top: 4px;
        z-index: -6;
        border-color: transparent;
        border-style: solid;
        border-width: 0 2px 2px 0;
        transition: background-color 0.2s, color 0.2s, border-color 0.2s;
    }

    .dashboard-btn:hover {
        background-color: var(--accent-secondary);
        color: var(--accent-primary);
        border-color: var(--bg-secondary);
    }
</style>
