<script lang="ts">
    import "$lib/palette.css";
    import "$lib/scrollbar.css";
    import type { Game, GameCatalog } from "$lib/api";

    interface Props {
        catalog: GameCatalog;
        selected?: string[];
        ondone: () => void;
    }

    let { catalog, selected = $bindable([]), ondone }: Props = $props();

    let spent = $derived(
        catalog.games
            .filter((g) => selected.includes(g.name))
            .reduce((sum, g) => sum + g.credits, 0),
    );
    let left = $derived(catalog.credit_budget - spent);
    let retired = $derived(
        catalog.games.length
            ? [
                  ...new Set(
                      selected.filter(
                          (n) => !catalog.games.some((g) => g.name === n),
                      ),
                  ),
              ]
            : [],
    );

    function toggle(game: Game) {
        selected = selected.includes(game.name)
            ? selected.filter((n) => n !== game.name)
            : [...selected, game.name];
    }
</script>

<div class="games-pane">
    <div class="pane-header">
        <div class="credit-bar">
            {#each Array(catalog.credit_budget) as _, i (i)}
                <span class="segment" class:filled={i < left}></span>
            {/each}
        </div>
        <div class="header-row">
            <p class="credits-left">
                {left}/{catalog.credit_budget} Credits Left
            </p>
            <button type="button" class="done" onclick={ondone}>Done</button>
        </div>
    </div>

    <div class="grid themed-scroll">
        {#each retired as name (name)}
            <div class="tile chosen">
                <p class="tile-name">{name}</p>
                <p class="placeholder">No longer offered</p>
                <div class="tile-footer">
                    <span class="cost">Free to remove</span>
                    <button
                        type="button"
                        class="toggle"
                        aria-label={`Remove ${name}`}
                        onclick={() =>
                            (selected = selected.filter((n) => n !== name))}
                        >−</button
                    >
                </div>
            </div>
        {/each}
        {#each catalog.games as game (game.name)}
            {@const chosen = selected.includes(game.name)}
            {@const affordable = chosen || game.credits <= left}
            <div class="tile" class:chosen class:unaffordable={!affordable}>
                <p class="tile-name">{game.name}</p>
                <img src={game.image_url} alt={game.name} loading="lazy" />
                <div class="tile-footer">
                    <span class="cost"
                        >{game.credits} credit{game.credits === 1 ? "" : "s"}
                        <span class="copies">({game.copies_label})</span></span
                    >
                    <button
                        type="button"
                        class="toggle"
                        disabled={!affordable}
                        aria-label={`${chosen ? "Remove" : "Add"} ${game.name}`}
                        onclick={() => toggle(game)}
                        >{chosen ? "−" : "+"}</button
                    >
                </div>
            </div>
        {/each}
    </div>
</div>

<style>
    .games-pane {
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        height: 100%;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
        overflow: hidden;
    }

    .pane-header {
        padding: 16px;
        border-bottom: 4px solid var(--border);
    }
    .credit-bar {
        display: flex;
        gap: 6px;
    }
    .segment {
        flex: 1;
        height: 16px;
        background-color: var(--bg-secondary);
        border: 3px solid var(--border);
        border-radius: 6px;
        transition: background-color 0.2s ease;
    }
    .segment.filled {
        background-color: var(--accent-primary);
    }
    .header-row {
        display: grid;
        grid-template-columns: 1fr auto 1fr;
        align-items: center;
        margin-top: 12px;
    }
    .credits-left {
        grid-column: 2;
        margin: 0;
        color: var(--text-primary);
        font-size: 1.5rem;
        font-weight: bold;
    }
    .done {
        grid-column: 3;
        justify-self: end;
        padding: 6px 16px;
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
        border: 4px solid var(--border);
        border-radius: 6px;
        font-family: inherit;
        font-size: 1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .done:hover {
        background-color: var(--accent-secondary);
    }

    .grid {
        flex: 1;
        overflow-y: auto;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        align-content: start;
        gap: 12px;
        padding: 16px;
    }
    .tile {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 8px;
        background-color: var(--bg-secondary);
        border: 3px solid var(--border);
        border-radius: 8px;
        transition:
            border-color 0.2s ease,
            opacity 0.2s ease;
    }
    .tile.chosen {
        border-color: var(--accent-primary);
    }
    .tile.unaffordable {
        opacity: 0.45;
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
        color: var(--text-primary);
        font-weight: bold;
    }
    .tile img,
    .placeholder {
        display: block;
        width: 100%;
        aspect-ratio: 460 / 215;
        object-fit: cover;
        border-radius: 4px;
        background-color: var(--bg-tertiary);
    }
    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0;
        color: var(--text-secondary);
    }
    .tile-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    .cost {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }
    .copies {
        opacity: 0.75;
    }
    .toggle {
        width: 32px;
        height: 32px;
        padding: 0;
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
        border: 3px solid var(--border);
        border-radius: 6px;
        font-family: inherit;
        font-size: 1.2rem;
        font-weight: bold;
        line-height: 1;
        cursor: pointer;
        transition: background-color 0.2s ease;
    }
    .tile.chosen .toggle {
        background-color: var(--bg-tertiary);
        color: var(--text-primary);
    }
    .toggle:hover:not(:disabled) {
        background-color: var(--accent-secondary);
        color: var(--text-on-accent);
    }
    .toggle:disabled {
        cursor: default;
    }
</style>
