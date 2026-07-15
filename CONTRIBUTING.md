# Contributing to Z3rm

Thank you for helping make Z3rm better!

All activity in z3rm forums and repositories is subject to our Code of Conduct.
Contributors should sign a Contributor License Agreement before substantial
contributions are merged.

## Contribution ideas

Z3rm is a focused fork of Zed, oriented around a GPU-rendered terminal,
built-in multiplexer, read-only file/diff viewer, and QuickJS extensions. We
welcome contributions that fit that scope.

In particular **we love PRs that are**:

- Fixing or extending the **docs**.
- Fixing **bugs**.
- **Small** enhancements to existing features to **make them work for more people**.
- **Small** extra features, like keybindings, actions, or extension hooks.
- Features we **explicitly called out as open to community contributions**.

**Thinking about proposing or building a larger feature? Don't start with a PR.**
Open a discussion issue first so we can agree on direction and integration
points.

## Sending changes

The z3rm culture values working code and focused conversations over long
discussion threads.

The best way to propose a change (excluding new features) is to send a pull
request. We will review as soon as we can. **Pinging maintainers by username or
email does not bump the priority of a particular PR.**

If you need help deciding how to fix a bug, please open a PR early so we can
discuss the change with code in hand.

**If you'd like your PR to have the best chance of being merged**:

- Make sure the change is **desired**: we are always happy to accept bugfixes,
  but **features should be confirmed with us first** if you want to avoid wasted
  effort.
- Include a clear description of **what you're solving**, and why it matters.
- Include **tests**. For UI changes, consider updating visual regression tests
  where applicable.
- If the change is visible in the UI, attach **screenshots or screen recordings**.
- Make the PR about **one thing only**.
- Keep AI assistance under your judgement and responsibility: we will not merge
  a PR whose author does not understand the changes.

## Things we will (probably) not merge

- Anything that should be provided by an extension rather than core.
- New file icons or hand-designed theme assets submitted without prior discussion.
- Features where the extra complexity is not worth the benefit.
- Giant refactorings.
- Non-trivial changes with no tests.
- Stylistic code changes that do not alter app logic.
- Anything that appears AI-generated without human understanding.

### AI Policy

We welcome the use of LLMs for coding, but we hold a high bar for all
contributions, and **we expect a human in the loop who genuinely understands the
work an LLM produces** on their behalf. For that reason, we **don't accept
contributions from autonomous agents**. Pull requests that appear to violate
this may be closed.

When communicating with maintainers, write in your own words. If you use an LLM
to translate or polish messages, quote the machine translation and include the
original text.

### Internal advice for reviewers

- If the fix/feature is obviously great, and the code is great. Hit merge.
- If the fix/feature is obviously great, and the code is nearly great. Send PR
  comments, or offer to pair to get things perfect.
- If the fix/feature is not obviously great, or the code needs rewriting from
  scratch. Close the PR with a thank you and some explanation.

### UI/UX checklist

When your changes affect UI, consult this checklist:

**Accessibility / Ergonomics**

- Do all keyboard shortcuts work as intended?
- Are shortcuts discoverable (tooltips, menus, docs)?
- Is it usable without a mouse?
- Do all mouse actions work (drag, context menus, resizing, scrolling)?
- Does the feature look great in light and dark mode themes?
- Are hover states and focus indicators clear and consistent?

**Responsiveness**

- Does the UI scale gracefully on narrow/short panes and high-DPI displays?
- Does resizing panes or windows keep the UI usable?
- Do dialogs or modals stay centered and within viewport bounds?

**Platform Consistency**

- Is the feature fully usable on Windows, Linux, and macOS?
- Does it respect system-level settings (fonts, scaling, input methods)?

**Performance**

- All user interactions must have instant feedback; slow work should show progress.
- Does it handle large files, big projects, or heavy workloads without degrading?
- Frames must take no more than 8 ms (120 fps).

**Consistency**

- Does it match z3rm's design language (spacing, typography, icons)?
- Are terminology, labels, and tone consistent?
- Are interactions consistent (tabs, modals, errors)?

**Internationalization & Text**

- Are strings concise, clear, and unambiguous?
- Do we avoid internal jargon that only insiders would know?

**User Paths & Edge Cases**

- What does the happy path look like?
- What does the unhappy path look like?
- How does it work offline vs. online?
- How does it behave if data is missing, corrupted, or delayed?
- Are error messages actionable?

**Discoverability & Learning**

- Can a first-time user figure it out without docs?
- Is there an intuitive way to undo/redo actions?
- Are power features discoverable but not intrusive?

## Bird's-eye view of Z3rm

Z3rm is made up of several smaller crates. Here are the ones you are most
likely to interact with:

- [`gpui`](/crates/gpui) is the GPU-accelerated UI framework. **Start here if you
  are new to the codebase.**
- [`terminal`](/crates/terminal) and [`terminal_view`](/crates/terminal_view)
  implement terminal emulation and rendering.
- [`workspace`](/crates/workspace) handles pane management and local state
  serialization.
- [`editor`](/crates/editor) provides the read-only file/diff viewer.
- [`project`](/crates/project) manages files and worktrees.
- [`theme`](/crates/theme) defines the theme system.
- [`ui`](/crates/ui) is the component library.
- [`cli`](/crates/cli) is the command-line launcher.
- [`z3rm`](/crates/z3rm) is the main entry crate that wires everything
  together.

## Packaging Z3rm

See the development docs for platform-specific packaging notes.
