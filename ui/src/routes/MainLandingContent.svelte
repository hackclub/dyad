<script>
    import "$lib/palette.css";
</script>

<div class="main-landing">
    <div class="description-entry">
        <p class="description-title">What is this?</p>
        <p class="description-text">
            This is a You Ship, We Ship program by <a
                href="https://hackclub.com/"
                class="desc-link">Hack Club</a
            > where you and a friend ship a software project together. Then, we'll
            send you your pick of a two player game for the two of you to play!
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">Who is this for?</p>
        <p class="description-text">
            Anyone 13-18, inclusive. Get your friends to join, even if they've
            never been a part of Hack Club before!
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">What sorts of projects can I build?</p>
        <p class="description-text">
            Anything that's software! Make a website, or a game, or something
            else entirely. It just has to be built together with your friend,
            and you've both got to hit your hour targets.
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">Can I have started my project earlier?</p>
        <p class="description-text">
            Yep! Dyad will accept a project from a pair regardless of when that
            project was created. The only rule is that the project cannot have
            been previously approved by another Hack Club YSWS program.
        </p>
        <p class="description-text">
            Some programs may claim a part of your project ahead of time. Dyad
            cannot accept that previously claimed time, but can accept time over
            and above. For instance, you may not be able to submit Third Space
            projects from past weeks. Still a chance though, and you'll be let
            know if you need to log more time.
        </p>
        <p class="description-text">
            Though I guess one more thing: if another YSWS program rejected your
            project because you tried to cheat the time tracking systems, please
            don't submit here. You won't get any prizes and it's just more work
            for the reviewers.
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">What are the hour targets?</p>
        <p class="description-text">
            You have to each spend five (5) hours <span style:font-weight="bold"
                >each</span
            >
            on the project, tracked with
            <a href="https://hackatime.hackclub.com/" class="desc-link"
                >Hackatime</a
            >
            or
            <a href="https://lapse.hackclub.com/" class="desc-link">Lapse</a>.
            However, if it's the first time coding and/or participating in a
            Hack Club YSWS program for one of you, that person only has to do
            three (3) hours. Other person still has to do the full five, though.
            Make sure you sync Lapse with Hackatime if you're going to use it!
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">
            Can I have more than two people or go solo?
        </p>
        <p class="description-text">
            Nope, any given project submitted to Dyad must be made by exactly
            two people. However, you can submit as many projects as you would
            like, and you can switch up who you're working with between
            projects.
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">What if one member gets in trouble?</p>
        <p class="description-text">
            If one member winds up banned from Hackatime or Slack, the other
            member of the team unfortunately cannot receive a standard prize. We
            only pursue bans in extraordinary circumstances, such as deliberate
            attempts to mislead the review process. The other team member may
            discretionarily get some sort of solo prize, evaluated on a
            case-by-case basis.
        </p>
    </div>
    <div class="description-entry">
        <p class="description-title">I have more questions!</p>
        <p class="description-text">
            Ask in the Slack Channel <a
                href="https://hackclub.enterprise.slack.com/archives/C0BUXJ7T15F"
                class="desc-link">#dyad</a
            >
            or shoot
            <a href="mailto:brendan@hackclub.com" class="desc-link">me</a> an email.
        </p>
    </div>
</div>

<style>
    .main-landing {
        background-color: var(--bg-tertiary);
        margin-top: calc(10vh + 4px);
        width: 80vw;
        border-top-right-radius: calc(4vh - 4px);
        border-bottom-right-radius: calc(4vh - 4px);
        z-index: 1;
        position: relative;
        border-color: var(--bg-secondary);
        border-style: solid;
        border-width: 0 4px 4px 0;
        display: grid;
        grid-template-columns: repeat(auto-fill, min(400px, 60vw));
        justify-content: center;
        gap: 32px;
        padding-top: 6vh;
        padding-bottom: 6vh;
    }

    .description-entry {
        width: min(400px, 60vw);
    }

    .description-title {
        color: var(--text-secondary);
        font-size: 32px;
    }

    .description-text {
        color: var(--text-primary);
        font-size: 18px;
    }

    .desc-link {
        color: var(--accent-text);
    }
</style>
