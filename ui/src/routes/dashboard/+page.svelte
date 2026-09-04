<script>
    import "$lib/palette.css";

    function new_project() {
        alert("New project");
    }
</script>

<div class="top-bar">
    <div class="container">
        <h1>Dyad Dashboard</h1>
    </div>
    <button class="new-project-button container" onclick={new_project}
        ><h1>Add Project</h1></button
    >
</div>

<div class="container project-container">

</div>

<style>
    :root {
        margin-left: 10vw;
        margin-right: 10vw;
        margin-top: 5vh;
        color: var(--text-primary);
    }
    .container {
        box-sizing: border-box;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
        padding: 8px;
        width: fit-content;
    }
    .top-bar {
        width: 100%;
        display: flex;
        justify-content: space-between;
        align-items: stretch;
    }
    .new-project-button {
        color: var(--text-primary);
        transition: background-color 0.3s ease;
    }
    .new-project-button:hover {
        background-color: var(--accent-primary);
    }
    .project-container {
        display: flex;
        flex-wrap: wrap;
        gap: 16px;
        width: 100%;
        min-height: 60vh;
        margin-top: 5vh;

    }
</style>
