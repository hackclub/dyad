<script>
    import "$lib/palette.css";
    import { onMount } from "svelte";
    import BackgroundLines from "./BackgroundLines.svelte";
    import MainLandingContent from "./MainLandingContent.svelte";

    let login_failed = $state(false);

    onMount(() => {
        if (!new URLSearchParams(location.search).has("login_error")) return;
        login_failed = true;
        history.replaceState(null, "", "/");
    });
</script>

<svelte:head>
    <title>Dyad</title>
</svelte:head>

<BackgroundLines />

{#if login_failed}
    <button class="login-error" onclick={() => (login_failed = false)}
        >Couldn't finish signing you in. Try again in a bit.</button
    >
{/if}

<!-- Title -->
<div class="title">
    <h3 class="title-subtext">Hack Club</h3>
    <h1 class="title-text">DYAD</h1>
    <h1 class="explainer-text">
        Build a project with a friend, get a game to play together!
    </h1>
</div>

<!-- Orpheus -->
<a href="https://hackclub.com/"
    ><img
        style="position: absolute; top: 0; right: 2.5vw; border: 0; width: 15vw; z-index: 999;"
        src="https://assets.hackclub.com/flag-orpheus-top.svg"
        alt="Hack Club"
    /></a
>

<!-- Dashboard button -->
<a href="/login" class="dashboard-btn">Open Dashboard</a>

<!-- Little nub -->
<div class="little-nub"></div>

<!-- Main content -->
<MainLandingContent></MainLandingContent>

<footer class="site-footer">
    <a href="https://hackclub.com/privacy-and-terms" style="color: #993100;"
        >Hack Club Privacy & Terms</a
    >
    <p style="margin-left: 3vw; margin-right: 3vw">
        Made by <a
            href="https://hackclub.enterprise.slack.com/team/U08LEHUPSCR"
            style="color: #993100;">Brendan</a
        > @ Hack Club HQ. By teens, for teens.
    </p>
    <a href="https://forms.hackclub.com/bounty" style="color: #993100;"
        >Fulfillment Bounty</a
    >
    <a
        href="https://security.hackclub.com"
        style="color: #993100; margin-left: 3vw;">Security Bounty</a
    >
</footer>

<style>
    :root {
        overflow-x: hidden;
        overscroll-behavior: none;
    }

    .login-error {
        position: fixed;
        top: 16px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 1000;
        padding: 8px 16px;
        border: 4px solid var(--border);
        border-radius: 16px;
        background-color: var(--bg-tertiary);
        color: var(--text-primary);
        font: inherit;
        cursor: pointer;
    }

    .title {
        height: 65vh;
        width: calc(80vw + 4px);
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

    .explainer-text {
        color: var(--text-dark);
        margin-left: 8px;
        margin-right: 8px;
    }

    .dashboard-btn {
        color: var(--text-on-accent);
        text-decoration: none;
        background-color: var(--accent-primary);
        width: calc(min(400px, 60vw) - 4px);
        height: calc(10vh - 1px);
        font-size: 4cqmin;
        line-height: 10vh;
        display: block;
        text-align: center;
        border-top-right-radius: calc(4vh - 6px);
        border-bottom-right-radius: 5vh;
        position: absolute;
        margin-top: 4px;
        border-color: transparent;
        border-style: solid;
        border-width: 0 2px 2px 0;
        transition:
            background-color 0.2s,
            color 0.2s,
            border-color 0.2s;
    }

    .dashboard-btn:hover {
        background-color: var(--accent-secondary);
        border-color: var(--bg-secondary);
    }

    .little-nub {
        background-color: var(--bg-tertiary);
        width: 10vh;
        height: calc(5vh + 1px);
        left: calc(min(400px, 60vw) - 5vh - 2px);
        margin-top: calc(5vh + 4px);
        z-index: -7;
        position: absolute;
        -webkit-mask-image: radial-gradient(
            circle 5vh at top right,
            transparent 98%,
            black 100%
        );
        mask-image: radial-gradient(
            circle 5vh at top right,
            transparent 98%,
            black 100%
        );
    }

    .site-footer {
        height: 80px;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: var(--bg-primary);
        color: var(--text-dark);
    }
</style>
