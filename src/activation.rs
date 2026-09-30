use gpui_kit::*;

/// Brings `window` forward for another launch. Wayland compositors only move
/// focus for a request that carries a valid activation token, so the token
/// the launcher gave that launch is used when there is one.
pub fn raise(window: AnyWindowHandle, token: Option<String>, cx: &mut App) {
    cx.activate(true);
    window
        .update(cx, |_, window, _| {
            if !token.is_some_and(|token| raise_with_token(window, &token)) {
                window.activate_window();
            }
        })
        .ok();
}

#[cfg(not(target_os = "linux"))]
fn raise_with_token(_: &Window, _: &str) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn raise_with_token(window: &Window, token: &str) -> bool {
    wayland::activate(window, token).is_some()
}

#[cfg(target_os = "linux")]
mod wayland {
    use std::sync::Mutex;

    use gpui_kit::Window;
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    use wayland_backend::client::{Backend, ObjectId};
    use wayland_client::globals::{GlobalListContents, registry_queue_init};
    use wayland_client::protocol::wl_registry::{self, WlRegistry};
    use wayland_client::protocol::wl_surface::WlSurface;
    use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{
        self, XdgActivationV1,
    };

    struct Activator;

    struct Activation {
        connection: Connection,
        queue: EventQueue<Activator>,
        global: XdgActivationV1,
    }

    static ACTIVATION: Mutex<Option<Activation>> = Mutex::new(None);

    /// Asks the compositor to focus `window` with `token`. GPUI keeps its
    /// Wayland objects private, so this binds `xdg_activation_v1` through a
    /// second event queue on GPUI's display. The display and surface come
    /// from GPUI's live window, and the queue sits in a static that is never
    /// dropped, because libwayland keeps routing its registry's events there.
    pub fn activate(window: &Window, token: &str) -> Option<()> {
        let RawWindowHandle::Wayland(surface) =
            HasWindowHandle::window_handle(window).ok()?.as_raw()
        else {
            return None;
        };
        let mut activation = ACTIVATION.lock().ok()?;
        if activation.is_none() {
            *activation = connect(window);
        }
        let activation = activation.as_mut()?;
        let surface =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }
                .ok()?;
        let surface = WlSurface::from_id(&activation.connection, surface).ok()?;
        activation.global.activate(token.to_owned(), &surface);
        activation.queue.roundtrip(&mut Activator).ok()?;
        Some(())
    }

    fn connect(window: &Window) -> Option<Activation> {
        let RawDisplayHandle::Wayland(display) =
            HasDisplayHandle::display_handle(window).ok()?.as_raw()
        else {
            return None;
        };
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<Activator>(&connection).ok()?;
        let global = globals.bind(&queue.handle(), 1..=1, ()).ok()?;
        Some(Activation {
            connection,
            queue,
            global,
        })
    }

    impl Dispatch<WlRegistry, GlobalListContents> for Activator {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: wl_registry::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<XdgActivationV1, ()> for Activator {
        fn event(
            _: &mut Self,
            _: &XdgActivationV1,
            _: xdg_activation_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
}
