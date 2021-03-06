pub mod git;
pub mod githost;

use std::future::Future;
use std::sync::Mutex;
use tokio::{runtime::Runtime, sync::mpsc};

pub type AsyncActionCallback = Box<dyn Fn(String) -> ()>;

pub enum AsyncActionResult {
    Result(Result<String, String>),
    Callback(String),
}

pub type AsyncActionResultSend = dyn Fn(AsyncActionResult) -> ();

// pub type Action = dyn Fn(&str) -> Box<dyn Future<Output = ()>>;
pub type AsyncAction<'a> =
    dyn Fn(&'a str, &'a AsyncActionResultSend) -> Box<dyn Future<Output = ()> + 'a>;
pub type SyncAction<'a> = dyn Fn(&'a str) -> Result<String, String>;

pub struct Callbacks {
    pub ok: AsyncActionCallback,
    pub err: AsyncActionCallback,
    pub callback: AsyncActionCallback,
}

/*
    Async actions can do one of three things.
    1. Return an Ok(result). This closes the callback context and no further responses are allowed.
    2. Return an Err(err).. This closes the callback context and no further responses are allowed.
    3. Return a Callback(data). This doesn't close the channel.

    We'd have preferred not blocking, but Android JVM can't handle callbacks from arbitrary threads.
    But still it isn't too bad. We're being called by Java threadpool threads.

    A future update could be to attach the threadpool threads to the JVM on Android.
    This can potentially avoid blocking on the action.
*/

pub fn handle_async(action: &str, args: &str, callbacks: Callbacks, runtime: &Mutex<Runtime>) {
    let maybe_action_handler =
        git::get_async_handler(action).or(githost::get_async_handler(action));

    match maybe_action_handler {
        Some(Action) => {
            let (tx, mut rx) = mpsc::unbounded_channel::<AsyncActionResult>();

            let send = |result: AsyncActionResult| ();
            let boxed_send: Box<dyn Fn(AsyncActionResult) -> ()> = Box::new(send);

            loop {
                let msg = runtime.lock().unwrap().block_on(async { rx.recv().await });

                match msg {
                    Some(AsyncActionResult::Result(Ok(msg_txt))) => {
                        (callbacks.ok)(msg_txt);
                        break;
                    }
                    Some(AsyncActionResult::Result(Err(msg_txt))) => {
                        (callbacks.err)(msg_txt);
                        break;
                    }
                    Some(AsyncActionResult::Callback(msg_txt)) => {
                        (callbacks.callback)(msg_txt);
                    }
                    None => {
                        break;
                    }
                }
            }
        }
        None => {
            (callbacks.err)(format!(
                "{{ ok: false, error: \"The sync action {action} was unhandled.\" }}",
                action = action
            ));
        }
    }
}

/*
    Sync actions can do one of three things.
    1. Return an Ok(result). This closes the callback context and no further responses are allowed.
    2. Return an Err(err).. This closes the callback context and no further responses are allowed.
*/
pub fn handle_sync(action: &str, args: &str) -> Result<String, String> {
    let maybe_action_handler = git::get_sync_handler(action).or(githost::get_sync_handler(action));

    match maybe_action_handler {
        Some(action_handler) => {
            let result = action_handler(args);
            match result {
                Ok(result_success) => Ok(format!(
                    "{{ ok: true, result: {result} }}",
                    result = result_success
                )),
                Err(err) => Err(format!("{{ error: {err}.\" }}", err = err)),
            }
        }
        None => Err(format!(
            "{{ ok: false, error: \"The sync action {action} was unhandled.\" }}",
            action = action
        )),
    }
}
