use std::future::Future;
use std::pin::Pin;

type ChannelFuture = Box<dyn Future<Output = ()> + Send>;

#[derive(Debug)]
pub enum JoinError {
    /// The future was dropped, likely due to a panic.
    FutureDropped,
    /// The [`JoinHandle`] was joined already.
    DoubleJoin,
}

impl From<flume::RecvError> for JoinError {
    fn from(_: flume::RecvError) -> Self {
        Self::FutureDropped
    }
}

pub struct JoinHandle<T> {
    recv: flume::Receiver<T>,
    joined: bool,
}

impl<T: 'static + Send> JoinHandle<T> {
    fn new_with_future<F>(fut: F) -> (Self, Box<dyn Future<Output = ()> + Send>)
    where
        F: Future<Output = T> + Send + 'static,
    {
        let (sender, recv) = flume::bounded(1);
        let fut = Box::new(async move {
            let res = fut.await;
            sender.send(res).ok();
        });
        (
            Self {
                recv,
                joined: false,
            },
            fut,
        )
    }

    pub fn join(self) -> Result<T, JoinError> {
        if self.joined {
            Err(JoinError::DoubleJoin)
        } else {
            Ok(self.recv.recv()?)
        }
    }

    pub fn try_join(&mut self) -> Result<Option<T>, JoinError> {
        let result = self.recv.try_recv();
        match result {
            Ok(data) => Ok(Some(data)),
            Err(err) => {
                if self.joined {
                    Err(JoinError::DoubleJoin)
                } else if matches!(err, flume::TryRecvError::Disconnected) {
                    Err(JoinError::FutureDropped)
                } else {
                    // No data available
                    self.joined = true;
                    Ok(None)
                }
            }
        }
    }

    pub async fn join_async(self) -> Result<T, JoinError> {
        if self.joined {
            Err(JoinError::DoubleJoin)
        } else {
            Ok(self.recv.recv_async().await?)
        }
    }
}

pub struct Runtime {
    sender: flume::Sender<ChannelFuture>,
}

impl Runtime {
    pub fn spawn<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (handle, fut) = JoinHandle::new_with_future(fut);
        self.sender.send(fut).expect("Internal runtime error!");
        handle
    }

    pub fn single_thread() -> Self {
        Self::launch(1)
    }

    pub fn multi_thread(num_of_threads: usize) -> Result<Self, ()> {
        if num_of_threads < 1 {
            Err(())
        } else {
            Ok(Self::launch(num_of_threads))
        }
    }

    fn launch(num_of_threads: usize) -> Runtime {
        let (tx, rx) = flume::unbounded::<ChannelFuture>();

        for _ in 0..num_of_threads {
            let rx = rx.clone();
            std::thread::spawn(move || {
                thread_runtime(rx);
            });
        }

        Runtime { sender: tx }
    }
}

fn thread_runtime(rx: flume::Receiver<ChannelFuture>) {
    block_on(async move {
        while let Ok(fut) = rx.recv_async().await {
            spawn(Pin::from(fut));
        }
    });
}

fn thread_context() -> glib::MainContext {
    glib::MainContext::thread_default().unwrap_or_else(glib::MainContext::new)
}

fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    thread_context().block_on(future)
}

fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    thread_context().spawn_local_with_priority(glib::PRIORITY_HIGH, future);
}
