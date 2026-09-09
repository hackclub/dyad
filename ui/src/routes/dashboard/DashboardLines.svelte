<script lang="ts">
  import { onMount } from "svelte";

  onMount(() => {
    const canvas_element = document.getElementById("dashboard-lines");
    const line_colour = getComputedStyle(
      document.documentElement,
    ).getPropertyValue("--bg-secondary");
    const unit_line_distance = window.innerHeight / 20;

    function draw() {
      let canvas = canvas_element as HTMLCanvasElement;
      if (canvas === null) {
        return;
      }
      canvas.width = window.innerWidth;
      canvas.height = document.documentElement.scrollHeight;

      const ctx = canvas.getContext("2d");
      if (ctx === null) {
        return;
      }

      function drawStraightLine(
        ctx: CanvasRenderingContext2D,
        x0: number,
        y0: number,
        x1: number,
        y1: number,
      ) {
        ctx.beginPath();
        ctx.moveTo(x0, y0);
        ctx.lineTo(x1, y1);
        ctx.strokeStyle = line_colour;
        ctx.lineWidth = 4;
        ctx.stroke();
      }

      function verticalSegment(
        ctx: CanvasRenderingContext2D,
        x: number,
        y: number,
      ) {
        drawStraightLine(ctx, x, y, x, y + unit_line_distance);
      }

      function horizontalSegment(
        ctx: CanvasRenderingContext2D,
        x: number,
        y: number,
      ) {
        drawStraightLine(ctx, x, y, x + unit_line_distance, y);
      }

      function drawArc(
        ctx: CanvasRenderingContext2D,
        center_x: number,
        center_y: number,
        starting_angle: number,
        ending_angle: number,
      ) {
        ctx.beginPath();
        ctx.arc(
          center_x,
          center_y,
          unit_line_distance,
          starting_angle,
          ending_angle,
        );
        ctx.strokeStyle = line_colour;
        ctx.lineWidth = 4;
        ctx.stroke();
      }

      function rightToDown(
        ctx: CanvasRenderingContext2D,
        x: number,
        y: number,
      ) {
        drawArc(ctx, x, y + unit_line_distance, -Math.PI / 2, 0);
      }

      function leftToDown(ctx: CanvasRenderingContext2D, x: number, y: number) {
        drawArc(ctx, x, y + unit_line_distance, -Math.PI / 2, -Math.PI);
      }

      function leftToUp(ctx: CanvasRenderingContext2D, x: number, y: number) {
        drawArc(ctx, x, y - unit_line_distance, Math.PI / 2, 0);
      }

      function rightToUp(ctx: CanvasRenderingContext2D, x: number, y: number) {
        drawArc(ctx, x, y - unit_line_distance, -Math.PI, -Math.PI / 2);
      }

      horizontalSegment(ctx, 100, 300);
      rightToDown(ctx, 100 + unit_line_distance, 300);
      leftToDown(ctx, 100 + unit_line_distance * 2, 300);
      leftToUp(ctx, 100 + unit_line_distance * 3, 300);
      rightToUp(ctx, 100 + unit_line_distance * 4, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 5, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 6, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 7, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 8, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 9, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 10, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 11, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 12, 300);
      horizontalSegment(ctx, 100 + unit_line_distance * 13, 300);

      verticalSegment(ctx, 100, 300);
      verticalSegment(ctx, 100 + unit_line_distance, 300);
      verticalSegment(ctx, 100 + unit_line_distance * 2, 300);
      verticalSegment(ctx, 100 + unit_line_distance * 3, 300);
      verticalSegment(ctx, 100 + unit_line_distance * 4, 300);
    }

    draw();
    window.addEventListener("resize", draw);
    return () => window.removeEventListener("resize", draw);
  });
</script>

<canvas id="dashboard-lines"> </canvas>

<style>
  #dashboard-lines {
    margin: 0;
    position: absolute;
    top: 0;
    left: 0;
    z-index: -1;
  }
</style>
