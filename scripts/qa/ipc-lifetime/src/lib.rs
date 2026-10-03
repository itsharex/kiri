//! Headless lifetime tests for the GObject ownership shape used by Wry IPC.
//! No GTK, WebKit, browser, application profile, display, or event loop is used.
//! The ownership under test is GObject ownership; Rc stores observations and
//! the removable application-owner slot used by the reentrant-dispose test.

#[cfg(test)]
mod tests {
    use gio::prelude::*;
    use glib::subclass::prelude::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Default)]
    pub struct Observed {
        dispose_calls: Cell<usize>,
        finalizations: Cell<usize>,
        callbacks: Cell<usize>,
        rejected: Cell<usize>,
        closure_drops: Cell<usize>,
        delivered: RefCell<Vec<(String, String)>>,
    }

    mod imp {
        use super::*;

        #[derive(Default)]
        pub struct View {
            pub manager: RefCell<Option<gio::SimpleAction>>,
            pub observed: RefCell<Rc<Observed>>,
            pub uri: RefCell<String>,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for View {
            const NAME: &'static str = "KiriWeakIpcLifetimeHarnessView";
            type Type = super::View;
        }

        impl ObjectImpl for View {
            fn dispose(&self) {
                let observed = self.observed.borrow();
                observed.dispose_calls.set(observed.dispose_calls.get() + 1);
                // This is a normal, idempotent GObject dispose implementation.
                self.manager.borrow_mut().take();
            }
        }

        // glib calls the Rust implementation's Drop from GObject finalization.
        // Unlike a weak-notify callback, this is a finalization observation.
        impl Drop for View {
            fn drop(&mut self) {
                let observed = self.observed.borrow();
                observed.finalizations.set(observed.finalizations.get() + 1);
            }
        }
    }

    glib::wrapper! {
        pub struct View(ObjectSubclass<imp::View>);
    }

    struct ClosureProbe(Rc<Observed>);
    impl Drop for ClosureProbe {
        fn drop(&mut self) {
            self.0.closure_drops.set(self.0.closure_drops.get() + 1);
        }
    }

    fn fixture() -> (View, gio::SimpleAction, Rc<Observed>) {
        let observed = Rc::new(Observed::default());
        let manager = gio::SimpleAction::new("ipc", Some(glib::VariantTy::STRING));
        let view: View = glib::Object::new();
        *view.imp().observed.borrow_mut() = observed.clone();
        *view.imp().uri.borrow_mut() = "kiri://localhost/?window=library".into();
        *view.imp().manager.borrow_mut() = Some(manager.clone());
        (view, manager, observed)
    }

    fn attach_weak_handler(view: &View, manager: &gio::SimpleAction, observed: &Rc<Observed>) {
        // Same strong-to-weak conversion and guarded upgrade as the Wry patch.
        let webview = view.downgrade();
        let observed = observed.clone();
        let probe = ClosureProbe(observed.clone());
        manager.connect_activate(move |_manager, msg| {
            let _keep_probe_in_closure = &probe;
            observed.callbacks.set(observed.callbacks.get() + 1);
            let Some(webview) = webview.upgrade() else {
                observed.rejected.set(observed.rejected.get() + 1);
                return;
            };
            if let Some(msg) = msg.and_then(|m| m.str()) {
                observed
                    .delivered
                    .borrow_mut()
                    .push((webview.imp().uri.borrow().clone(), msg.into()));
            }
        });
    }

    fn emit(manager: &gio::SimpleAction, payload: &str) {
        manager.activate(Some(&payload.to_variant()));
    }

    #[test]
    fn normal_ipc_delivers_uri_and_body_without_retaining_temporary_upgrade() {
        let (view, manager, observed) = fixture();
        let before = view.ref_count();
        attach_weak_handler(&view, &manager, &observed);
        assert_eq!(view.ref_count(), before, "connecting must not add a view owner");
        emit(&manager, r#"{"cmd":"get_library"}"#);
        emit(&manager, "second-message");
        assert_eq!(observed.callbacks.get(), 2);
        assert_eq!(observed.rejected.get(), 0);
        assert_eq!(
            *observed.delivered.borrow(),
            vec![
                ("kiri://localhost/?window=library".into(), r#"{"cmd":"get_library"}"#.into()),
                ("kiri://localhost/?window=library".into(), "second-message".into()),
            ]
        );
        assert_eq!(view.ref_count(), before, "callback upgrade must be temporary");
        drop(view);
        assert_eq!(observed.finalizations.get(), 1);
        drop(manager);
        assert_eq!(observed.closure_drops.get(), 1);
    }

    #[test]
    fn disposal_invalidates_weak_reference_while_strong_owner_still_exists() {
        let (view, manager, observed) = fixture();
        attach_weak_handler(&view, &manager, &observed);
        let weak = view.downgrade();
        assert!(weak.upgrade().is_some());
        // SAFETY: This harness owns the custom subclass, whose dispose method
        // is idempotent. No disposed view methods are invoked afterwards;
        // only the still-live, separately-owned manager emits a signal.
        unsafe { view.run_dispose() };
        assert!(observed.dispose_calls.get() >= 1);
        assert_eq!(observed.finalizations.get(), 0, "strong owner is still in scope");
        assert!(weak.upgrade().is_none(), "GWeakRef clears at dispose, before finalize");
        emit(&manager, "late-after-dispose");
        assert_eq!(observed.callbacks.get(), 1, "the signal really reached the closure");
        assert_eq!(observed.rejected.get(), 1);
        assert!(observed.delivered.borrow().is_empty());
        drop(view);
        assert_eq!(observed.finalizations.get(), 1);
        drop(manager);
        assert_eq!(observed.closure_drops.get(), 1);
    }

    #[test]
    fn late_callbacks_after_finalization_are_safe_no_ops() {
        let (view, manager, observed) = fixture();
        attach_weak_handler(&view, &manager, &observed);
        let weak = view.downgrade();
        drop(view);
        assert_eq!(observed.finalizations.get(), 1);
        assert!(weak.upgrade().is_none());
        assert_eq!(observed.closure_drops.get(), 0, "manager still owns signal closure");
        emit(&manager, "late-one");
        emit(&manager, "late-two");
        assert_eq!(observed.callbacks.get(), 2);
        assert_eq!(observed.rejected.get(), 2);
        assert!(observed.delivered.borrow().is_empty());
        drop(manager);
        assert_eq!(observed.closure_drops.get(), 1);
    }

    #[test]
    fn dropping_all_external_owners_finalizes_view_and_manager_without_cycle() {
        let (view, manager, observed) = fixture();
        attach_weak_handler(&view, &manager, &observed);
        let weak_view = view.downgrade();
        let weak_manager = manager.downgrade();
        drop(manager);
        assert!(weak_manager.upgrade().is_some(), "view owns the manager");
        drop(view);
        assert!(weak_view.upgrade().is_none());
        assert!(weak_manager.upgrade().is_none());
        assert_eq!(observed.finalizations.get(), 1);
        assert_eq!(observed.closure_drops.get(), 1);
    }

    #[test]
    fn reentrant_dispose_during_app_handler_returns_safely_and_rejects_next_signal() {
        let (view, manager, observed) = fixture();
        let weak_view = view.downgrade();
        let callback_view = view.downgrade();
        // This slot models the application's removable strong GObject owner.
        // Rc shares the slot only; it does not model view lifetime semantics.
        let external_owner = Rc::new(RefCell::new(Some(view)));
        let handler_owner = external_owner.clone();
        let handler_observed = observed.clone();
        let handler_weak = weak_view.clone();
        let disposed_in_handler = Rc::new(Cell::new(false));
        let weak_cleared_in_handler = Rc::new(Cell::new(false));
        let finalizations_before_handler_return = Rc::new(Cell::new(usize::MAX));
        let did_dispose = disposed_in_handler.clone();
        let weak_cleared = weak_cleared_in_handler.clone();
        let finalizations_at_return = finalizations_before_handler_return.clone();
        let app_handler = move |request: (String, String)| {
            // The URI/body are owned values, built before invoking app code.
            handler_observed.delivered.borrow_mut().push(request);
            if let Some(owner) = handler_owner.borrow_mut().take() {
                // SAFETY: This is the harness-owned, idempotently disposable
                // subclass. No view methods run after this explicit dispose.
                unsafe { owner.run_dispose() };
                did_dispose.set(handler_observed.dispose_calls.get() >= 1);
                weak_cleared.set(handler_weak.upgrade().is_none());
                // Now only the callback's temporary GObject upgrade remains.
                drop(owner);
                finalizations_at_return.set(handler_observed.finalizations.get());
            }
        };
        let callback_observed = observed.clone();
        let probe = ClosureProbe(observed.clone());
        manager.connect_activate(move |_manager, msg| {
            let _keep_probe_in_closure = &probe;
            callback_observed.callbacks.set(callback_observed.callbacks.get() + 1);
            let Some(webview) = callback_view.upgrade() else {
                callback_observed.rejected.set(callback_observed.rejected.get() + 1);
                return;
            };
            if let Some(msg) = msg.and_then(|m| m.str()) {
                let request = (webview.imp().uri.borrow().clone(), msg.to_owned());
                app_handler(request);
                // Match Wry: no view method is used after the app callback.
                // The temporary strong ref delays finalization, not dispose.
            }
        });

        emit(&manager, "cancel-capture");
        assert!(disposed_in_handler.get(), "temporary upgrade must not prevent dispose");
        assert!(weak_cleared_in_handler.get());
        assert!(external_owner.borrow().is_none(), "app owner was released reentrantly");
        assert_eq!(finalizations_before_handler_return.get(), 0,
            "temporary upgraded owner keeps the allocation alive during app code");
        assert_eq!(observed.finalizations.get(), 1,
            "return from callback must release its temporary upgraded owner");
        assert!(weak_view.upgrade().is_none());
        assert_eq!(*observed.delivered.borrow(), vec![(
            "kiri://localhost/?window=library".into(), "cancel-capture".into()
        )]);
        emit(&manager, "late-after-reentrant-dispose");
        assert_eq!(observed.callbacks.get(), 2);
        assert_eq!(observed.rejected.get(), 1);
        assert_eq!(observed.delivered.borrow().len(), 1);
        drop(manager);
        assert_eq!(observed.closure_drops.get(), 1);
    }

    #[test]
    fn strong_capture_negative_control_retains_cycle_until_handler_disconnected() {
        let (view, manager, observed) = fixture();
        let strong_view = view.clone();
        let probe = ClosureProbe(observed.clone());
        let handler = manager.connect_activate(move |_manager, _msg| {
            let _keep_probe_in_closure = &probe;
            // Force capture of an actual strong GObject reference.
            let _uri = strong_view.imp().uri.borrow().clone();
        });
        let weak_view = view.downgrade();
        let weak_manager = manager.downgrade();
        drop(view);
        drop(manager);
        assert!(weak_view.upgrade().is_some(), "baseline ownership cycle must be observable");
        assert!(weak_manager.upgrade().is_some());
        assert_eq!(observed.dispose_calls.get(), 0);
        assert_eq!(observed.finalizations.get(), 0);
        assert_eq!(observed.closure_drops.get(), 0);

        // Clean up the intentional negative-control cycle in the same test.
        let manager = weak_manager.upgrade().unwrap();
        manager.disconnect(handler);
        assert!(weak_view.upgrade().is_none());
        assert_eq!(observed.finalizations.get(), 1);
        assert_eq!(observed.closure_drops.get(), 1);
        drop(manager);
        assert!(weak_manager.upgrade().is_none());
    }
}
