<!-- top bit -->
<div
    class="hline bg-line-item"
    style="width: 12vw; top: 5vh; left: 0;
border-bottom-right-radius: 2px;"
></div>
<div
    class="hline bg-line-item"
    style="width: 15vw; top: 5vh; left: 18vw; border-bottom-left-radius: 2px;
    border-bottom-right-radius: 2px;"
></div>
<div class="dot bg-line-item" style="top: 5vh; left: 35vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 36vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 37vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 38vw;"></div>
<div class="dot bg-line-item" style="top: 5vh; left: 39vw;"></div>
<div
    class="hline bg-line-item"
    style="width: calc(39vw + 8px - 5vh + 2px); top: 5vh; left: 41vw; border-bottom-left-radius: 2px;"
></div>
<div class="ter-bg" style="width: 100vw; height: 5vh;"></div>
<div
    class="ter-bg"
    style="top: 5vh; left: calc(80vw + 4px); width: 20vw; height: 5vh;"
></div>
<div
    class="corner-patch"
    style="top: 5vh; left: calc(80vw + 8px - 4vh); width: 4vh; height: 4vh;"
></div>

<!-- right thing -->
<div
    class="tr-turn bg-line-item turn-box"
    style="left: calc(80vw + 8px - 5vh); top: 5vh; height: calc(5vh + 2px);"
></div>
<div class="tr-secondary bg-line-item"></div>

<!-- mid-bottom thing -->
<div class="hline bg-line-item" style="width: calc(min(400px, 60vw) - 5vh + 2px); top: 70vh; left: 0;"></div>
<div
    class="tr-turn bg-line-item turn-box"
    style="top: 70vh; left: calc(min(400px, 60vw) - 5vh + 2px); height: calc(5vh + 2px);"
></div>
<div
    class="bl-turn bg-line-item turn-box"
    style="top: 75vh; left: calc(min(400px, 60vw) - 2px);"
></div>
<div
    class="hline bg-line-item"
    style="width: calc((80vw + 4px) - (min(400px, 60vw) + 5vh - 6px) - 5vh + 4px); top: 80vh; left: calc(min(400px, 60vw) + 5vh - 6px);"
></div>
<div
    class="tr-turn bg-line-item turn-box"
    style="top: 80vh; left: calc(80vw + 4px - 5vh);"
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
        border-bottom-left-radius: 2px;
        border-bottom-right-radius: 2px;
    }

    .hline {
        height: 4px;
    }

    .tr-secondary {
        top: calc(10vh);
        left: calc(80vw + 4px);
        width: 20vw;
        height: 70vh;
        border-bottom-left-radius: 80vh;
        box-shadow:
            inset 0 -4px 0 0 var(--bg-secondary),
            inset 4px 0 0 0 var(--bg-secondary);

        background-color: var(--bg-tertiary);
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
    }

    .bl-turn {
        height: calc(5vh + 4px);
        width: 5vh;
        box-shadow:
            inset 0 -4px 0 0 var(--bg-secondary),
            inset 4px 0 0 0 var(--bg-secondary);
        border-bottom-left-radius: 5vh;
    }

    .ter-bg {
        position: absolute;
        background-color: var(--bg-tertiary);
        z-index: -6;
    }

    .corner-patch {
        position: absolute;
        background-color: var(--bg-tertiary);
        z-index: -6;
        -webkit-mask-image: radial-gradient(
            circle 4vh at bottom left,
            transparent 98%,
            black 100%
        );
        mask-image: radial-gradient(
            circle 4vh at bottom left,
            transparent 98%,
            black 100%
        );
    }
</style>
