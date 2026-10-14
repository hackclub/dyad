<script lang="ts">
    import "$lib/palette.css";
    import { onMount } from "svelte";
    import DashboardLines from "../dashboard/DashboardLines.svelte";
    import { api, type GameCatalog } from "$lib/api";

    let catalog = $state<GameCatalog | null>(null);
    let error = $state("");

    onMount(async () => {
        try {
            catalog = await api<GameCatalog>("/dashboard/games");
        } catch (e) {
            error = (e as Error).message;
        }
    });
</script>

<svelte:head>
    <title>Dyad Games</title>
</svelte:head>

<DashboardLines />

<main>
    <header class="card">
        <h1>Dyad Games</h1>
        <p>
            Ship a project with a friend and pick from these two player games
            for the two of you to play. <a href="/">Back to Dyad</a>
        </p>
    </header>

    {#if catalog}
        <div class="grid">
            {#each catalog.games as game (game.name)}
                <div class="tile">
                    <p class="tile-name">{game.name}</p>
                    <img src={game.image_url} alt={game.name} loading="lazy" />
                </div>
            {/each}
        </div>
    {:else if error}
        <p class="card">{error}</p>
    {/if}
</main>

<style>
    :global(body) {
        color: var(--text-primary);
    }

    main {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        max-width: 64rem;
        margin: 0 auto;
        padding: 2rem 1rem;
    }

    .card {
        box-sizing: border-box;
        margin: 0;
        padding: 1.25rem;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
    }

    h1 {
        margin: 0;
        font-size: 2rem;
        font-weight: 700;
    }

    header p {
        margin: 0.5rem 0 0;
        color: var(--text-secondary);
    }

    header a {
        color: var(--accent-text);
    }

    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
        gap: 12px;
    }

    .tile {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 8px;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
    }

    .tile-name {
        margin: 0;
        height: 2.5em;
        line-height: 1.25;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        font-weight: bold;
    }

    .tile img {
        display: block;
        width: 100%;
        aspect-ratio: 460 / 215;
        object-fit: cover;
        border-radius: 8px;
        background-color: var(--bg-secondary);
    }
</style>
