<!-- top bit -->
<div class="hline bg-line-item" style="width: 12vw; top: 5vh; left: 0;"></div>
<div
    class="hline bg-line-item"
    style="width: 15vw; top: 5vh; left: 18vw;"
></div>
<div class="dot bg-line-item" style="top: 5vh; left: 35vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 36vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 37vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 38vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 39vw;"></div>
<div
    class="hline bg-line-item"
    style="width: calc(29vw + 8px - 5vh); top: 5vh; left: 41vw;"
></div>

<!-- right thing -->
<div
    class="tr-turn bg-line-item turn-box"
    style="left: calc(70vw + 8px - 5vh); top: 5vh;"
></div>
<div class="tr-secondary bg-line-item"></div>

<!-- mid-bottom thing -->
<div class="hline bg-line-item" style="width: 30vw; top: 70vh; left: 0;"></div>
<div class="tr-turn bg-line-item turn-box" style="top: 70vh; left: 30vw;"></div>
<div
    class="bl-turn bg-line-item turn-box"
    style="top: 75vh; left: calc(30vw + 5vh - 4px);"
></div>
<div
    class="hline bg-line-item"
    style="width: calc((70vw + 4px) - (30vw + 10vh) - 5vh + 4px); top: 80vh; left: calc(30vw + 10vh - 4px);"
></div>
<div
    class="tr-turn bg-line-item turn-box"
    style="top: 80vh; left: calc(70vw + 4px - 5vh);"
></div>
<div
    class="vline bg-line-item"
    style="height: 80vh; top: 85vh; left: 70vw;"
></div>

<style>
    .bg-line-item {
        background-color: var(--bg-secondary);
        z-index: -5;
        position: absolute;
    }

    .dot {
        height: 4px;
        width: 4px;
    }

    .hline {
        height: 4px;
    }

    .vline {
        width: 4px;
    }

    .tr-secondary {
        top: calc(10vh);
        left: calc(70vw + 4px);
        width: 30vw;
        height: 70vh;
        border-bottom-left-radius: 80vh;
        box-shadow:
            inset 0 -4px 0 0 var(--bg-secondary),
            inset 4px 0 0 0 var(--bg-secondary);
        transform: translateZ(0);

        background-color: transparent;
    }

    .turn-box {
        background-color: transparent;
    }

    .tr-turn {
        height: 5vh;
        width: 5vh;
        box-shadow:
            inset 0 4px 0 0 var(--bg-secondary),
            inset -4px 0 0 0 var(--bg-secondary);
        border-top-right-radius: 4vh;
        transform: translateZ(0);
    }

    .bl-turn {
        height: calc(5vh + 4px);
        width: 5vh;
        box-shadow:
            inset 0 -4px 0 0 var(--bg-secondary),
            inset 4px 0 0 0 var(--bg-secondary);
        border-bottom-left-radius: 5vh;
        transform: translateZ(0);
    }
</style>
