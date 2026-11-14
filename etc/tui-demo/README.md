# TUI demo videos

`make video` (`etc/demo/demo record`) records each chapter, `<name>.tape`, in light and dark,
in the order of their names, which start with a number, to `out/<name>-light.mp4` and
`out/<name>-dark.mp4`. It needs ffmpeg, a betamax with `Announce`, `KeyboardOverlayFontSize` and
`KeyboardOverlayColor` (betamax 0.3.0 has none of them), and `target/release/delta` built against the
gitu fork. `make video` does not build delta.

Each recording runs in its own clone of this repository at a fixed commit, with its own HOME, both
made by `setup`. The HOME's git config is `demo.gitconfig`, which holds the owner's delta and TUI
settings, the themes in `themes.gitconfig` (its own, not this repository's), and
`light.gitconfig` or `dark.gitconfig`, which set delta's light or dark mode, since delta cannot ask
betamax's terminal for its background. Nothing comes from the user's own git config, so a recording
does not change as the user's config does.

A chapter is split into scenes at lines `# scene`. Each scene is recorded on its own, after the
scenes before it are replayed hidden and quickly, so scenes record in parallel, and a scene whose
tape and inputs are unchanged comes from the cache in `work/scenes/` instead of being recorded
again. Put a `# scene` before each step, so that changing a step records only its scene.
A scene marked `# scene other-theme` is recorded with the other theme, the terminal's and
delta's, so that a chapter can show what the other kind of background looks like; the scene
after it is back in the chapter's theme.
`THEMES=light` records the light video only.

`_settings.tape` is shared by every chapter: font, terminal size, key chips. `_light.tape` and
`_dark.tape` set the terminal theme. Tapes starting with `_` are not chapters.

## How a chapter is written

- End with the git config the chapter uses: `show_config` in `env.sh` prints `demo.gitconfig` as
  a viewer would put it in their own, with any lines the chapter uses besides.
- Say what is about to happen, then do it: an `Announce` before each step, in a few words, naming
  keys as delta's manual does (`shift+tab`). Do not use `Caption`.
  `demo` pauses after each announcement, so the viewer sees the screen before the step changes it.
- Hold a result only as long as it takes to see it: 1.5s after an unfold or fold.
- Do not explain what needs no explaining, such as moving up and down, enter opening, or q and
  esc going back. Move with the arrow keys,
  and do not mention `j`/`k` or `alt+j`/`alt+k`.
- Start where the user's own session starts. Side-by-side is on from the start: each chapter
  presses `S` while hidden, once the TUI is open.
- Keep each announcement to one or two lines, and do not let a URL wrap.
- Wait for text on the screen (`Wait+Screen`) before acting on anything git or delta produce,
  rather than sleeping and hoping.
- Watch both videos before committing: frames from both themes, at each announcement and each key.

## Known problems

- After `S` is toggled a few times in a commit opened from the log, `q` returns to the log with the
  cursor on a different commit.
- `L` changes nothing while side-by-side is on, since side-by-side always shows line numbers, so no
  chapter toggles it yet.

## Recording your voice

Each `Announce` can be spoken over the video. What it says is a fragment, identified by a hash of
its text, so editing an announcement's text makes your recording of it out of date. Each fragment
has a slot: the time its announcement is held, `Announce@3s "…"`, or 2 seconds without a duration.
Speech longer than its slot holds the announcement, and so the video, for longer.

1. See what to record: `etc/demo/demo voice-list` (or `voice-list 03-log.tape` for one chapter) prints
   each fragment's id, slot, state (`missing`, `ok`, or over its slot) and text.
2. Record: `etc/demo/demo voice-record` goes through each fragment without a recording. It shows
   the text and the slot; press Enter, speak, press Enter. It records from the default microphone
   to `voice/me/<id>.wav` and says if you overran the slot.
3. Redo one: delete its `voice/me/<id>.wav` and run `voice-record` again.
4. More or less time to speak at a point: change the duration of that `Announce` in the tape, e.g.
   `Announce@4s "…"`. Only the slot changes; the recording stays valid since the text did not.
5. Check that a video has every fragment: `voice-list` shows none `missing`;
   `make video VOICE=me` also refuses to start if any is missing, and lists it.
6. Add the voice: `make video VOICE=me`, or `make video VOICE=me THEMES=light` for the light video
   only. Scenes whose tape, recordings and inputs are unchanged come from the cache.

`VOICE=elevenlabs` speaks with an ElevenLabs voice instead, and needs `ELEVENLABS_API_KEY`; its
audio is cached under `voice/elevenlabs/`. `voice/` is not tracked by git.
