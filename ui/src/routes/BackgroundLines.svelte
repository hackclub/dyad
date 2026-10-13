<script>
    let rect = $state();

    const g = $derived.by(() => {
        const w = rect?.width ?? 0;
        const h = rect?.height ?? 0;
        if (!w || !h) return null;
        const vw = w / 100;
        const vh = h / 100;
        const m = Math.min(400, 60 * vw);
        const big = Math.min(20 * vw, 70 * vh);
        const rx = 80 * vw + 4;
        const y1 = 5 * vh;

        /** @param {number} a @param {number} b */
        const bar = (a, b) =>
            `M${a} ${y1}H${b}V${y1 + 2}A2 2 0 0 1 ${b - 2} ${y1 + 4}H${a + 2}A2 2 0 0 1 ${a} ${y1 + 2}Z`;
        const segs = [
            [0, 12 * vw],
            [18 * vw, 33 * vw],
            ...[35, 36, 37, 38, 39].map((i) => [i * vw, i * vw + 4]),
        ];
        const bumps = segs
            .slice()
            .reverse()
            .map(([a, b]) => `H${b}V${y1 + 2}H${a}V${y1}`)
            .join("");

        return {
            frame:
                `M0 0H${w}V${80 * vh - 2}H${rx + big}A${big - 2} ${big - 2} 0 0 1 ${rx + 2} ${80 * vh - big}` +
                `V${9 * vh}A${4 * vh - 2} ${4 * vh - 2} 0 0 0 ${rx + 4 - 4 * vh} ${y1 + 2}H${41 * vw}V${y1}${bumps}Z`,
            lower:
                `M0 ${80 * vh + 2}H${m - 2 - 5 * vh}V${74 * vh}H${m}V${75 * vh + 4}` +
                `A${5 * vh - 2} ${5 * vh - 2} 0 0 0 ${m - 2 + 5 * vh} ${80 * vh + 2}H${rx - 4 * vh}` +
                `A${4 * vh - 2} ${4 * vh - 2} 0 0 1 ${rx - 2} ${84 * vh}V${84 * vh + 4}H0Z`,
            bars: [...segs, [41 * vw, 41 * vw + 6]].map(([a, b]) => bar(a, b)).join(""),
            right:
                `M${41 * vw + 2} ${y1 + 2}H${rx + 4 - 4 * vh}A${4 * vh - 2} ${4 * vh - 2} 0 0 1 ${rx + 2} ${9 * vh}` +
                `V${80 * vh - big}A${big - 2} ${big - 2} 0 0 0 ${rx + big} ${80 * vh - 2}H${w}`,
            hug:
                `M0 ${70 * vh + 3}H${m + 4 - 4 * vh}A${4 * vh - 5} ${4 * vh - 5} 0 0 1 ${m - 1} ${74 * vh - 2}V${75 * vh + 6}`,
            s:
                `M0 ${70 * vh + 2}H${m + 4 - 4 * vh}A${4 * vh - 4} ${4 * vh - 4} 0 0 1 ${m} ${74 * vh - 2}V${75 * vh + 4}` +
                `A${5 * vh - 2} ${5 * vh - 2} 0 0 0 ${m - 2 + 5 * vh} ${80 * vh + 2}H${rx - 4 * vh}` +
                `A${4 * vh - 2} ${4 * vh - 2} 0 0 1 ${rx - 2} ${84 * vh}V${h}`,
        };
    });
</script>

<div class="bg-lines" bind:contentRect={rect}>
    {#if g}
        <svg>
            <path class="fill" d={g.frame} />
            <path class="fill" d={g.lower} />
            <path class="ink" d={g.bars} />
            <path class="line" d={g.right} />
            <path class="line hug" d={g.hug} />
            <path class="line" d={g.s} />
        </svg>
    {/if}
</div>

<style>
    .bg-lines {
        position: absolute;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        z-index: -5;
        pointer-events: none;
    }

    svg {
        display: block;
        width: 100%;
        height: 100%;
        animation: fade-in 0.3s ease-out;
    }

    .fill {
        fill: var(--bg-tertiary);
    }

    .ink {
        fill: var(--bg-secondary);
    }

    .line {
        fill: none;
        stroke: var(--bg-secondary);
        stroke-width: 4;
    }

    .hug {
        stroke-width: 6;
    }

    @keyframes fade-in {
        from {
            opacity: 0;
        }
    }
</style>
