use crate::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Window,
};

/// Builds a `Deferred` element, which delays the layout and paint of its child.
pub fn deferred(child: impl IntoElement) -> Deferred {
    Deferred {
        child: Some(child.into_any_element()),
        priority: 0,
    }
}

/// An element which delays the painting of its child until after all of
/// its ancestors, while keeping its layout as part of the current element tree.
pub struct Deferred {
    child: Option<AnyElement>,
    priority: usize,
}

impl Deferred {
    /// Sets the `priority` value of the `deferred` element, which
    /// determines the drawing order relative to other deferred elements,
    /// with higher values being drawn on top.
    pub fn with_priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl Element for Deferred {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<crate::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout_id = self.child.as_mut().unwrap().request_layout(window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let child = self.child.take().unwrap();
        let element_offset = window.element_offset();
        window.defer_draw(child, element_offset, self.priority, None)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

impl Deferred {
    /// Sets a priority for the element. A higher priority conceptually means painting the element
    /// on top of deferred draws with a lower priority (i.e. closer to the viewer).
    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{
        Context, Entity, StyleRefinement, TestAppContext, Window, anchored, deferred, div, point,
        prelude::*, px, size,
    };

    struct SubscriptionLeaf {
        value: usize,
        output: Rc<Cell<usize>>,
    }

    struct SubscriptionPanel {
        leaf: Entity<SubscriptionLeaf>,
        deferred_child: bool,
        renders: Rc<Cell<usize>>,
    }

    impl Render for SubscriptionPanel {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            let leaf = self.leaf.clone();
            let child = crate::canvas(
                |_, _, _| {},
                move |_, _, _, cx| {
                    let leaf = leaf.read(cx);
                    leaf.output.set(leaf.value);
                },
            )
            .size(px(50.));
            if self.deferred_child {
                deferred(child).into_any_element()
            } else {
                child.into_any_element()
            }
        }
    }

    struct SubscriptionRoot {
        panel: Entity<SubscriptionPanel>,
        deferred_panel: bool,
        renders: Rc<Cell<usize>>,
    }

    impl Render for SubscriptionRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            let panel = self
                .panel
                .clone()
                .cached(StyleRefinement::default().size(px(100.)));
            if self.deferred_panel {
                deferred(panel).into_any_element()
            } else {
                panel.into_any_element()
            }
        }
    }

    #[gpui::test]
    fn test_cached_reuse_keeps_entity_subscriptions(cx: &mut TestAppContext) {
        for deferred_child in [true, false] {
            for deferred_panel in [false, true] {
                let output = Rc::new(Cell::new(0));
                let renders = Rc::new(Cell::new(0));
                let root_renders = Rc::new(Cell::new(0));
                let leaf = cx.new(|_| SubscriptionLeaf {
                    value: 1,
                    output: output.clone(),
                });
                let panel = cx.new(|_| SubscriptionPanel {
                    leaf: leaf.clone(),
                    deferred_child,
                    renders: renders.clone(),
                });
                let window = cx.open_window(size(px(800.), px(600.)), |_, _| SubscriptionRoot {
                    panel,
                    deferred_panel,
                    renders: root_renders.clone(),
                });
                cx.run_until_parked();
                assert_eq!(output.get(), 1);
                let initial_renders = renders.get();
                for _ in 0..2 {
                    window
                        .update(cx, |_, _, cx| cx.notify())
                        .expect("notify root");
                    cx.run_until_parked();
                    assert_eq!(
                        renders.get(),
                        initial_renders,
                        "panel should reuse its cache"
                    );
                }

                let renders_before_notify = root_renders.get();
                leaf.update(cx, |leaf, cx| {
                    leaf.value = 2;
                    cx.notify();
                });
                cx.run_until_parked();
                let context =
                    format!("deferred child {deferred_child}, deferred panel {deferred_panel}");
                assert!(
                    root_renders.get() > renders_before_notify,
                    "leaf notification should redraw: {context}"
                );
                // A plain entity read has no dispatch-tree path to invalidate the
                // cached panel, so refresh to run the paint callback again.
                window
                    .update(cx, |_, window, _| window.refresh())
                    .expect("refresh canvas");
                cx.run_until_parked();
                assert_eq!(output.get(), 2, "{context}");
            }
        }
    }

    /// A stand-in for a dock panel hosting a popover (deferred draw) whose
    /// content opens another popover (a deferred draw created while
    /// prepainting the first one's content).
    struct PanelView;

    impl Render for PanelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().key_context("Panel").size_full().child(
                deferred(
                    anchored().position(point(px(10.), px(10.))).child(
                        div().key_context("Popover").w(px(200.)).h(px(200.)).child(
                            deferred(
                                anchored().position(point(px(30.), px(30.))).child(
                                    div()
                                        .key_context("NestedMenu")
                                        .debug_selector(|| "NESTED_MENU".into())
                                        .w(px(50.))
                                        .h(px(50.)),
                                ),
                            )
                            .with_priority(2),
                        ),
                    ),
                )
                .with_priority(1),
            )
        }
    }

    struct RootView {
        panel: Entity<PanelView>,
    }

    impl Render for RootView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().key_context("Root").size_full().child(
                self.panel
                    .clone()
                    .cached(StyleRefinement::default().size_full()),
            )
        }
    }

    /// Regression test for a crash with nested deferred draws (e.g. a popover
    /// menu inside a popover hosted by a cached dock panel). Prepaint indices
    /// recorded during the deferred draw rounds must index the same
    /// `deferred_draws` vector that `reuse_prepaint` slices on the next frame;
    /// previously they were measured against a transient per-round vector, so
    /// reusing the panel's subtree grafted the wrong deferred draws and
    /// panicked in the dispatch tree.
    #[gpui::test]
    fn test_nested_deferred_draws_with_reused_views(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(800.), px(600.)), |_, cx| {
            let panel = cx.new(|_| PanelView);
            RootView { panel }
        });
        cx.run_until_parked();

        let menu_bounds = window
            .update(cx, |_, window, _| {
                window
                    .rendered_frame
                    .debug_bounds
                    .get("NESTED_MENU")
                    .copied()
            })
            .unwrap()
            .expect("NESTED_MENU debug bounds not found");
        assert_eq!(menu_bounds.size, size(px(50.), px(50.)));

        // Re-render only the root view; the panel is cached, so its subtree -
        // including both deferred draw records - is reused from the previous
        // frame.
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.run_until_parked();

        // Reuse the subtree a second time, exercising ranges that were
        // themselves recorded during a reused frame.
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.run_until_parked();

        // Re-render the panel itself again to prove the popovers still draw.
        window
            .update(cx, |root, _, cx| {
                root.panel.update(cx, |_, cx| cx.notify());
            })
            .unwrap();
        cx.run_until_parked();

        window
            .update(cx, |_, window, _| {
                assert_eq!(window.rendered_frame.deferred_draws.len(), 2);
                assert!(
                    window
                        .rendered_frame
                        .debug_bounds
                        .contains_key("NESTED_MENU")
                );
            })
            .unwrap();
    }

    type DrawTrace = Rc<std::cell::RefCell<Vec<(&'static str, &'static str)>>>;

    fn traced_draw(label: &'static str, trace: DrawTrace) -> impl IntoElement {
        let prepaint_trace = trace.clone();
        crate::canvas(
            move |_, _, _| prepaint_trace.borrow_mut().push(("prepaint", label)),
            move |_, (), _, _| trace.borrow_mut().push(("paint", label)),
        )
        .size(px(24.))
    }

    struct CachedDeferredLayer {
        prompt: bool,
        trace: DrawTrace,
        renders: Rc<Cell<usize>>,
    }

    impl Render for CachedDeferredLayer {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            if self.prompt {
                div()
                    .key_context("PromptDeferred")
                    .size_full()
                    .child(
                        deferred(
                            div()
                                .size(px(100.))
                                .child(traced_draw("prompt-deferred", self.trace.clone()))
                                .child(
                                    deferred(
                                        div()
                                            .debug_selector(|| "PROMPT_NESTED".into())
                                            .size(px(24.))
                                            .child(traced_draw(
                                                "prompt-nested",
                                                self.trace.clone(),
                                            )),
                                    )
                                    .with_priority(1),
                                ),
                        )
                        .with_priority(0),
                    )
                    .into_any_element()
            } else {
                div()
                    .key_context("RootDeferred")
                    .size_full()
                    .child(
                        deferred(traced_draw("root-deferred", self.trace.clone()))
                            .with_priority(usize::MAX),
                    )
                    .into_any_element()
            }
        }
    }

    struct PromptTestRoot {
        focus: crate::FocusHandle,
        panel: Entity<CachedDeferredLayer>,
        trace: DrawTrace,
    }

    impl Render for PromptTestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("prompt-test-root")
                .size_full()
                .track_focus(&self.focus)
                .child(traced_draw("root", self.trace.clone()))
                .child(
                    self.panel
                        .clone()
                        .cached(StyleRefinement::default().size(px(100.))),
                )
        }
    }

    struct DeferredPrompt {
        focus: crate::FocusHandle,
        panel: Entity<CachedDeferredLayer>,
        trace: DrawTrace,
    }

    impl crate::EventEmitter<crate::PromptResponse> for DeferredPrompt {}

    impl crate::Focusable for DeferredPrompt {
        fn focus_handle(&self, _: &crate::App) -> crate::FocusHandle {
            self.focus.clone()
        }
    }

    impl Render for DeferredPrompt {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("deferred-prompt")
                .size_full()
                .track_focus(&self.focus)
                .child(traced_draw("prompt-background", self.trace.clone()))
                .child(
                    self.panel
                        .clone()
                        .cached(StyleRefinement::default().size(px(100.))),
                )
        }
    }

    #[gpui::test]
    fn test_prompt_nested_deferred_draws_preserve_layers_and_cached_ranges(
        cx: &mut TestAppContext,
    ) {
        let trace: DrawTrace = Default::default();
        let root_renders = Rc::new(Cell::new(0));
        let prompt_renders = Rc::new(Cell::new(0));
        let prompt_view: Rc<std::cell::RefCell<Option<Entity<DeferredPrompt>>>> =
            Default::default();

        cx.update({
            let trace = trace.clone();
            let prompt_renders = prompt_renders.clone();
            let prompt_view = prompt_view.clone();
            move |app| {
                app.set_prompt_builder(move |_, _, _, _, handle, window, app| {
                    let panel = app.new(|_| CachedDeferredLayer {
                        prompt: true,
                        trace: trace.clone(),
                        renders: prompt_renders.clone(),
                    });
                    let view = app.new(|cx| DeferredPrompt {
                        focus: cx.focus_handle(),
                        panel,
                        trace: trace.clone(),
                    });
                    *prompt_view.borrow_mut() = Some(view.clone());
                    handle.with_view(view, window, app)
                });
            }
        });

        let window = cx.open_window(size(px(400.), px(300.)), {
            let trace = trace.clone();
            let root_renders = root_renders.clone();
            move |window, cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                let panel = cx.new(|_| CachedDeferredLayer {
                    prompt: false,
                    trace: trace.clone(),
                    renders: root_renders,
                });
                PromptTestRoot {
                    focus,
                    panel,
                    trace,
                }
            }
        });
        cx.run_until_parked();

        let mut response = window
            .update(cx, |_, window, app| {
                window.prompt(
                    crate::PromptLevel::Warning,
                    "Restore newer changes?",
                    None,
                    &["Restore", "Cancel"],
                    app,
                )
            })
            .expect("open the real in-window prompt");

        cx.update_window(window.into(), |_, window, app| {
            trace.borrow_mut().clear();
            window.refresh();
            window.draw(app).clear(app);
            assert_eq!(
                trace.borrow().as_slice(),
                &[
                    ("prepaint", "root"),
                    ("prepaint", "root-deferred"),
                    ("prepaint", "prompt-background"),
                    ("prepaint", "prompt-deferred"),
                    ("prepaint", "prompt-nested"),
                    ("paint", "root"),
                    ("paint", "root-deferred"),
                    ("paint", "prompt-background"),
                    ("paint", "prompt-deferred"),
                    ("paint", "prompt-nested"),
                ],
                "each deferred child must prepaint and paint once in its host layer",
            );
        })
        .expect("draw a prompt with nested deferred children");

        let initial_root_renders = root_renders.get();
        let initial_prompt_renders = prompt_renders.get();
        for _ in 0..2 {
            window
                .update(cx, |_, _, cx| cx.notify())
                .expect("notify root");
            cx.run_until_parked();
            cx.update_window(window.into(), |_, window, app| {
                trace.borrow_mut().clear();
                window.draw(app).clear(app);
                assert!(window.has_active_prompt());
                assert_eq!(window.rendered_frame.deferred_draws.len(), 3);
                assert_eq!(
                    window.rendered_frame.debug_bounds["PROMPT_NESTED"].size,
                    size(px(24.), px(24.)),
                );
                assert_eq!(
                    trace.borrow().as_slice(),
                    &[
                        ("prepaint", "root"),
                        ("prepaint", "prompt-background"),
                        ("paint", "root"),
                        ("paint", "prompt-background"),
                    ],
                    "both deferred subtrees should replay their cached frame ranges",
                );
            })
            .expect("reuse cached ranges with the prompt still open");
            assert_eq!(root_renders.get(), initial_root_renders);
            assert_eq!(prompt_renders.get(), initial_prompt_renders);
        }

        let prompt = prompt_view.borrow().as_ref().expect("built prompt").clone();
        prompt.update(cx, |prompt, cx| {
            prompt.panel.update(cx, |_, cx| cx.notify());
        });
        cx.run_until_parked();
        cx.update_window(window.into(), |_, window, app| {
            window.draw(app).clear(app);
            assert!(
                window
                    .rendered_frame
                    .debug_bounds
                    .contains_key("PROMPT_NESTED")
            );
        })
        .expect("redraw the invalidated prompt subtree");
        assert!(prompt_renders.get() > initial_prompt_renders);
        assert_eq!(response.try_recv().expect("pending response"), None);

        prompt.update(cx, |_, cx| cx.emit(crate::PromptResponse(1)));
        cx.run_until_parked();
        assert_eq!(response.try_recv().expect("prompt response"), Some(1));
        window
            .update(cx, |root, window, _| {
                assert!(!window.has_active_prompt());
                assert!(root.focus.is_focused(window));
            })
            .expect("restore the root focus");
        cx.update_window(window.into(), |_, window, app| {
            window.draw(app).clear(app);
            assert_eq!(window.rendered_frame.deferred_draws.len(), 1);
        })
        .expect("remove the prompt and retain root deferred draws");
    }
}
