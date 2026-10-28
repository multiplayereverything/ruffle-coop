//! The navigator for a session: loads finish at wall-clock times, so a later load would complete
//! at a different frame on each PC. Only the first fetch (the game file itself, before any frame
//! runs) goes through; every later load and every socket is refused, the same way on every PC.

use async_channel::{Receiver, Sender};
use ruffle_core::backend::navigator::{
    ErrorResponse, NavigationMethod, NavigatorBackend, OwnedFuture, Request, SuccessResponse,
};
use ruffle_core::indexmap::IndexMap;
use ruffle_core::loader::Error;
use ruffle_core::socket::{ConnectionState, SocketAction, SocketHandle};
use std::cell::Cell;
use std::time::Duration;
use url::{ParseError, Url};

pub struct SessionNavigator<N> {
    inner: N,
    fetches_allowed: Cell<u32>,
}

impl<N: NavigatorBackend> SessionNavigator<N> {
    pub fn new(inner: N) -> Self {
        Self {
            inner,
            fetches_allowed: Cell::new(1),
        }
    }

    #[cfg(test)]
    pub fn fetches_allowed(&self) -> u32 {
        self.fetches_allowed.get()
    }
}

impl<N: NavigatorBackend> NavigatorBackend for SessionNavigator<N> {
    fn navigate_to_url(
        &self,
        url: &str,
        target: &str,
        vars_method: Option<(NavigationMethod, IndexMap<String, String>)>,
    ) {
        // opens the browser; nothing comes back into the game
        self.inner.navigate_to_url(url, target, vars_method);
    }

    fn fetch(&self, request: Request) -> OwnedFuture<Box<dyn SuccessResponse>, ErrorResponse> {
        let allowed = self.fetches_allowed.get();
        if allowed > 0 {
            self.fetches_allowed.set(allowed - 1);
            return self.inner.fetch(request);
        }
        tracing::warn!("Ruffle Together: refused a load during the session: {}", request.url());
        let url = request.url().to_string();
        Box::pin(async move {
            Err(ErrorResponse {
                url,
                error: Error::FetchError("loads are off during a Ruffle Together session".into()),
            })
        })
    }

    fn resolve_url(&self, url: &str) -> Result<Url, ParseError> {
        self.inner.resolve_url(url)
    }

    fn spawn_future(&mut self, future: OwnedFuture<(), Error>) {
        self.inner.spawn_future(future);
    }

    fn pre_process_url(&self, url: Url) -> Url {
        self.inner.pre_process_url(url)
    }

    fn connect_socket(
        &mut self,
        _host: String,
        _port: u16,
        _timeout: Duration,
        handle: SocketHandle,
        _receiver: Receiver<Vec<u8>>,
        sender: Sender<SocketAction>,
    ) {
        let _ = sender.try_send(SocketAction::Connect(handle, ConnectionState::Failed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruffle_core::backend::navigator::NullNavigatorBackend;
    use std::time::Duration;

    #[test]
    fn only_the_game_file_loads_and_sockets_are_refused() {
        let mut nav = SessionNavigator::new(NullNavigatorBackend::new());
        assert_eq!(nav.fetches_allowed(), 1);
        let _game = nav.fetch(Request::get("file:///game.swf".into()));
        assert_eq!(nav.fetches_allowed(), 0);
        let later = futures::executor::block_on(nav.fetch(Request::get("file:///level.xml".into())));
        assert!(later.is_err(), "a load after the game file must be refused");

        let (_to_socket, from_game) = async_channel::unbounded();
        let (to_game, from_socket) = async_channel::unbounded();
        nav.connect_socket(
            "example.com".into(),
            80,
            Duration::from_secs(1),
            SocketHandle::default(),
            from_game,
            to_game,
        );
        assert!(matches!(
            from_socket.try_recv(),
            Ok(SocketAction::Connect(_, ConnectionState::Failed))
        ));
    }
}
