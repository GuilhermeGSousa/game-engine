use futures_channel::oneshot;

use std::future::Future;
use std::future::IntoFuture;

#[allow(dead_code)]
pub struct Task<T>(oneshot::Receiver<T>);

impl<T: 'static> Task<T> {
    pub fn new(future: impl Future<Output = T> + 'static) -> Self {
        let (sender, receiver) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            let value = future.await;
            let _ = sender.send(value);
        });
        Self(receiver.into_future())
    }
}
