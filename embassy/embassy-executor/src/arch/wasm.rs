#[cfg(feature = "executor-interrupt")]
compile_error!("`executor-interrupt` is not supported with `arch-wasm`.");

#[cfg(feature = "executor-thread")]
pub use thread::*;
#[cfg(feature = "executor-thread")]
mod thread {

    use core::marker::PhantomData;

    #[cfg(feature = "nightly")]
    pub use embassy_macros::main_wasm as main;
    use js_sys::Promise;
    use wasm_bindgen::prelude::*;

    use crate::raw::util::UninitCell;
    use crate::raw::{Pender, PenderInner};
    use crate::{raw, Spawner};

    /// WASM executor, wasm_bindgen to schedule tasks on the JS event loop.
    pub struct Executor {
        inner: raw::Executor,
        ctx: &'static WasmContext,
        not_send: PhantomData<*mut ()>,
    }

    pub(crate) struct WasmContext {
        promise: Promise,
        closure: UninitCell<Closure<dyn FnMut(JsValue)>>,
    }

    #[derive(Copy, Clone)]
    pub(crate) struct ThreadPender(&'static WasmContext);

    impl ThreadPender {
        #[allow(unused)]
        pub(crate) fn pend(self) {
            let _ = self.0.promise.then(unsafe { self.0.closure.as_mut() });
        }
    }

    impl WasmContext {
        pub fn new() -> Self {
            Self {
                promise: Promise::resolve(&JsValue::undefined()),
                closure: UninitCell::uninit(),
            }
        }
    }

    impl Executor {
        /// Create a new Executor.
        pub fn new() -> Self {
            let ctx = &*Box::leak(Box::new(WasmContext::new()));
            Self {
                inner: raw::Executor::new(Pender(PenderInner::Thread(ThreadPender(ctx)))),
                not_send: PhantomData,
                ctx,
            }
        }

        /// Run the executor.
        ///
        /// The `init` closure is called with a [`Spawner`] that spawns tasks on
        /// this executor. Use it to spawn the initial task(s). After `init` returns,
        /// the executor starts running the tasks.
        ///
        /// To spawn more tasks later, you may keep copies of the [`Spawner`] (it is `Copy`),
        /// for example by passing it as an argument to the initial tasks.
        ///
        /// This function requires `&'static mut self`. This means you have to store the
        /// Executor instance in a place where it'll live forever and grants you mutable
        /// access. There's a few ways to do this:
        ///
        /// - a [StaticCell](https://docs.rs/static_cell/latest/static_cell/) (safe)
        /// - a `static mut` (unsafe)
        /// - a local variable in a function you know never returns (like `fn main() -> !`), upgrading its lifetime with `transmute`. (unsafe)
        pub fn start(&'static mut self, init: impl FnOnce(Spawner)) {
            unsafe {
                let executor = &self.inner;
                self.ctx.closure.write(Closure::new(move |_| {
                    executor.poll();
                }));
                init(self.inner.spawner());
            }
        }
    }
}
