<script lang="ts">
    import { onMount } from "svelte";

    let { density = 0.5 }: { density?: number } = $props();

    const LINE_WIDTH = 4;
    const DASH = [10, 10];
    const STRAIGHT_WEIGHT = 2;
    const MIN_STEPS = 4;
    const SEARCH_BUDGET = 400;
    const MAX_FAILURES = 200;
    const TO_DASHED = 0.08;
    const TO_SOLID = 0.25;

    type Dir = [number, number];
    type Step = { i: number; j: number; turn: Dir | null; dashed: boolean };

    let canvas: HTMLCanvasElement;

    function generate(cols: number, rows: number) {
        const width = cols + 1;
        const used_nodes = new Uint8Array(width * (rows + 1));
        const used_cells = new Uint8Array(cols * rows);
        const total = used_nodes.length;
        let filled = 0;

        const is_edge = (i: number, j: number) =>
            i === 0 || j === 0 || i === cols || j === rows;
        const node = (i: number, j: number) => j * width + i;

        const starts: [number, number, Dir][] = [];
        for (let i = 1; i < cols; i++)
            starts.push([i, 0, [0, 1]], [i, rows, [0, -1]]);
        for (let j = 1; j < rows; j++)
            starts.push([0, j, [1, 0]], [cols, j, [-1, 0]]);

        function options(d: Dir) {
            const moves: (Dir | null)[] = [null, [-d[1], d[0]], [d[1], -d[0]]];
            const weights = [STRAIGHT_WEIGHT, 1, 1];
            const order: (Dir | null)[] = [];
            while (moves.length) {
                let r = Math.random() * weights.reduce((a, b) => a + b);
                let k = 0;
                while ((r -= weights[k]) > 0) k++;
                order.push(moves.splice(k, 1)[0]);
                weights.splice(k, 1);
            }
            return order;
        }

        function walk(i0: number, j0: number, d0: Dir) {
            type Frame = {
                i: number;
                j: number;
                d: Dir;
                turn: Dir | null;
                cell: number;
                left: (Dir | null)[];
            };
            const stack: Frame[] = [
                { i: i0, j: j0, d: d0, turn: null, cell: -1, left: [null] },
            ];
            used_nodes[node(i0, j0)] = 1;
            let budget = SEARCH_BUDGET;

            while (stack.length && budget-- > 0) {
                const top = stack[stack.length - 1];
                if (!top.left.length) {
                    stack.pop();
                    used_nodes[node(top.i, top.j)] = 0;
                    if (top.cell >= 0) used_cells[top.cell] = 0;
                    continue;
                }
                const turn = top.left.shift()!;
                const d = turn ?? top.d;
                const i = top.i + top.d[0] + (turn ? turn[0] : 0);
                const j = top.j + top.d[1] + (turn ? turn[1] : 0);
                if (
                    i < 0 ||
                    j < 0 ||
                    i > cols ||
                    j > rows ||
                    used_nodes[node(i, j)]
                )
                    continue;
                const cell = turn
                    ? Math.min(i, top.i) + Math.min(j, top.j) * cols
                    : -1;
                if (turn && used_cells[cell]) continue;
                const edge = is_edge(i, j);
                if (edge && (turn || stack.length < MIN_STEPS)) continue;

                used_nodes[node(i, j)] = 1;
                if (cell >= 0) used_cells[cell] = 1;
                stack.push({
                    i,
                    j,
                    d,
                    turn,
                    cell,
                    left: edge ? [] : options(d),
                });
                if (edge) {
                    filled += stack.length;
                    let dashed = false;
                    return stack.map((f): Step => {
                        dashed =
                            Math.random() < (dashed ? 1 - TO_SOLID : TO_DASHED);
                        return { i: f.i, j: f.j, turn: f.turn, dashed };
                    });
                }
            }

            for (const f of stack) {
                used_nodes[node(f.i, f.j)] = 0;
                if (f.cell >= 0) used_cells[f.cell] = 0;
            }
            return null;
        }

        const lines: Step[][] = [];
        let failures = 0;
        while (filled < density * total && failures < MAX_FAILURES) {
            const [i, j, d] = starts[Math.floor(Math.random() * starts.length)];
            const line = used_nodes[node(i, j)] ? null : walk(i, j, d);
            if (line) lines.push(line);
            else failures++;
        }
        return lines;
    }

    function draw() {
        const width = window.innerWidth;
        const height = window.innerHeight;
        const unit = height / 20;
        const scale = Math.min(window.devicePixelRatio || 1, 2);

        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
        canvas.width = Math.round(width * scale);
        canvas.height = Math.round(height * scale);

        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.scale(scale, scale);

        const solid = new Path2D();
        const dashed = new Path2D();
        for (const line of generate(
            Math.ceil(width / unit),
            Math.ceil(height / unit),
        )) {
            for (let k = 1; k < line.length; k++) {
                const prev = line[k - 1];
                const step = line[k];
                const path = step.dashed ? dashed : solid;
                if (k === 1 || step.dashed !== prev.dashed)
                    path.moveTo(prev.i * unit, prev.j * unit);
                if (!step.turn) {
                    path.lineTo(step.i * unit, step.j * unit);
                    continue;
                }
                const [tx, ty] = step.turn;
                const dx = step.i - prev.i - tx;
                const dy = step.j - prev.j - ty;
                path.arc(
                    (prev.i + tx) * unit,
                    (prev.j + ty) * unit,
                    unit,
                    Math.atan2(-ty, -tx),
                    Math.atan2(dy, dx),
                    dx * ty - dy * tx < 0,
                );
            }
        }

        ctx.strokeStyle = getComputedStyle(document.documentElement)
            .getPropertyValue("--bg-secondary")
            .trim();
        ctx.lineWidth = LINE_WIDTH;
        ctx.stroke(solid);
        ctx.setLineDash(DASH);
        ctx.stroke(dashed);
    }

    onMount(() => {
        let timer: ReturnType<typeof setTimeout>;
        function schedule() {
            clearTimeout(timer);
            timer = setTimeout(draw, 150);
        }

        draw();
        window.addEventListener("resize", schedule);
        return () => {
            clearTimeout(timer);
            window.removeEventListener("resize", schedule);
        };
    });
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
    canvas {
        position: fixed;
        top: 0;
        left: 0;
        z-index: -1;
        pointer-events: none;
    }
</style>
