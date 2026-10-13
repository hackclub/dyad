<script lang="ts">
    import "$lib/palette.css";
    import { onMount } from "svelte";
    import DashboardLines from "../dashboard/DashboardLines.svelte";
    import Funnel from "./Funnel.svelte";
    import LineChart from "./LineChart.svelte";
    import { api, type Stats } from "$lib/api";

    const short_time = (date: string) =>
        new Date(date).toLocaleString(undefined, {
            month: "short",
            day: "numeric",
            hour: "numeric",
            minute: "2-digit",
        });

    const duration = (hours: number | null) =>
        hours === null
            ? "n/a"
            : hours < 48
              ? `${Math.round(hours)}h`
              : `${(hours / 24).toFixed(1)}d`;

    let data = $state<Stats | null>(null);
    let error = $state("");

    const now = $derived(data?.hours[data.hours.length - 1]);

    const ranges: [string, number, "day" | "week" | "all"][] = [
        ["24h", 24, "day"],
        ["7d", 24 * 7, "week"],
        ["All", Infinity, "all"],
    ];
    let range = $state(24 * 7);
    const hours = $derived(data?.hours.slice(-range) ?? []);
    const funnel = $derived(
        data?.funnel[ranges.find(([, value]) => value === range)?.[2] ?? "all"],
    );
    const hour_labels = $derived(hours.map((hour) => short_time(hour.hour)));
    const quick = $derived(
        data && [
            [
                ["Projects created", data.totals.projects_created.toLocaleString()],
                ["Pairs formed", data.totals.pairs_formed.toLocaleString()],
                ["Hours spent building", Math.round(data.totals.hours_building).toLocaleString()],
                ["Projects shipped", data.totals.projects_shipped.toLocaleString()],
            ],
            [
                ["Projects approved", data.totals.projects_approved.toLocaleString()],
                ["Hours approved", Math.round(data.totals.hours_approved).toLocaleString()],
                ["Awaiting review", data.totals.awaiting_review.toLocaleString()],
                ["Average wait in queue", duration(data.totals.average_wait_hours)],
                ["Average time to approve", duration(data.totals.average_approval_hours)],
            ],
        ],
    );
    const steps = $derived(
        funnel && [
            { label: "Signed in", count: funnel.signed_in },
            { label: "Linked Hackatime", count: funnel.linked_hackatime },
            { label: "Created a project", count: funnel.created_project },
            { label: "Paired up", count: funnel.paired },
            { label: "Logged time", count: funnel.logged_time },
            { label: "Shipped a project", count: funnel.shipped },
            { label: "Approved", count: funnel.approved },
        ],
    );

    const refresh_ms = 30_000;

    let refreshing = $state(false);
    let refreshed = $state(0);
    let clock = $state(Date.now());
    let attempted = 0;
    let timer = 0;

    const ago = $derived.by(() => {
        const seconds = Math.max(0, Math.floor((clock - refreshed) / 1000));
        return seconds < 60
            ? `${seconds}s ago`
            : `${Math.floor(seconds / 60)}m ${seconds % 60}s ago`;
    });

    function schedule() {
        clearTimeout(timer);
        if (document.hidden || refreshing) return;
        timer = window.setTimeout(refresh, Math.max(0, attempted + refresh_ms - Date.now()));
    }

    async function refresh() {
        clearTimeout(timer);
        attempted = Date.now();
        refreshing = true;
        error = "";
        try {
            data = await api<Stats>("/stats/data");
            refreshed = clock = Date.now();
        } catch (e) {
            error = e instanceof Error ? e.message : "Couldn't load stats";
        }
        refreshing = false;
        schedule();
    }

    onMount(() => {
        refresh();
        const tick = setInterval(() => (clock = Date.now()), 1000);
        return () => {
            clearTimeout(timer);
            clearInterval(tick);
        };
    });
</script>

<svelte:head>
    <title>Dyad Stats</title>
</svelte:head>

<svelte:document onvisibilitychange={schedule} />

<DashboardLines />

{#if data && now && quick && steps}
    <main>
        <header class="card">
            <h1>Dyad Stats</h1>
            <div class="controls">
                <div class="ranges">
                    {#each ranges as [label, value] (label)}
                        <button
                            type="button"
                            class:active={range === value}
                            aria-pressed={range === value}
                            onclick={() => (range = value)}>{label}</button
                        >
                    {/each}
                </div>
                <div class="refresh">
                    <span>Refreshed {ago}</span>
                    <button type="button" disabled={refreshing} onclick={refresh}
                        >Refresh</button
                    >
                </div>
            </div>
        </header>

        <div class="pair">
            <section class="card split">
                <div class="half">
                    <div class="label">Signed Up Users</div>
                    <div class="big">{now.signed_up.toLocaleString()}</div>
                    <LineChart
                        values={hours.map((hour) => hour.signed_up)}
                        labels={hour_labels}
                    />
                </div>
                <div class="half">
                    <div class="label">Projects Created</div>
                    <div class="big">{now.projects_created.toLocaleString()}</div>
                    <LineChart
                        values={hours.map((hour) => hour.projects_created)}
                        labels={hour_labels}
                    />
                </div>
            </section>
            <section class="card split">
                <div class="half">
                    <div class="label">Projects Shipped</div>
                    <div class="big">{now.shipped.toLocaleString()}</div>
                    <LineChart
                        values={hours.map((hour) => hour.shipped)}
                        labels={hour_labels}
                    />
                </div>
                <div class="half logged-in">
                    <div class="label">Logged In (last 24 hours)</div>
                    <div class="big">
                        {data.totals.logged_in_last_day.toLocaleString()}
                    </div>
                </div>
            </section>
        </div>

        {#each quick as row, i (i)}
            <section class="card quick">
                {#each row as [label, value] (label)}
                    <div>
                        <div class="value">{value}</div>
                        <div class="label small">{label}</div>
                    </div>
                {/each}
            </section>
        {/each}

        <section class="card">
            <div class="label">Funnel</div>
            <Funnel {steps} />
        </section>
    </main>
{:else if error}
    <div class="status card">
        <p>{error}</p>
        <button type="button" onclick={refresh}>Retry</button>
    </div>
{/if}

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

    header {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem 1rem;
    }

    .controls {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.75rem 1.5rem;
    }

    .refresh {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-variant-numeric: tabular-nums;
    }

    .ranges {
        display: flex;
        gap: 4px;
    }

    button {
        padding: 0.5rem 1rem;
        background-color: var(--bg-secondary);
        border: 2px solid var(--border);
        border-radius: 8px;
        color: var(--text-primary);
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        transition:
            background-color 0.3s,
            color 0.3s;
    }

    button:hover:not(:disabled),
    button:focus-visible,
    .ranges button.active {
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
    }

    button:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .ranges button {
        padding: 0.25rem 0.6rem;
        font-size: 0.85rem;
    }

    h1 {
        margin: 0;
        font-size: 2rem;
        font-weight: 700;
    }

    .pair {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }

    .card {
        box-sizing: border-box;
        min-width: 0;
        padding: 1.25rem;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
    }

    .half {
        display: flex;
        flex-direction: column;
        flex: 1;
    }

    .logged-in {
        flex: 0;
    }

    .split {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .label {
        margin-bottom: 0.5rem;
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .big {
        margin-bottom: 1rem;
        font-size: 3rem;
        font-weight: 700;
    }

    .quick {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(7rem, max-content));
        justify-content: space-between;
        gap: 1rem 0.75rem;
    }

    .value {
        font-size: 1.75rem;
        font-weight: 700;
    }

    .label.small {
        margin: 0;
        font-size: 0.85rem;
    }

    .status {
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.75rem;
        max-width: calc(100vw - 2rem);
    }

    .status p {
        margin: 0;
    }

    @media (max-width: 40rem) {
        .pair {
            grid-template-columns: 1fr;
        }
    }
</style>
