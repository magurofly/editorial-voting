use std::{future::Future, sync::{Arc, Mutex}, task::{Context, Poll}};

pub struct DatabaseMiddleware {
    client_mutex: Arc<Mutex<Option<tokio_postgres::Client>>>,
    handle: tokio::task::JoinHandle<()>,
}
impl DatabaseMiddleware {
    pub fn new() -> Self {
        let database_url = std::env::var("EDITORIAL_VOTING_DATABASE_URL").unwrap();
        let client_mutex = Arc::new(Mutex::new(None));
        let handle = {
            let client_mutex_in_pooling_thread = client_mutex.clone();
            tokio::spawn(async move {
                let client_mutex = client_mutex_in_pooling_thread;
                loop {
                    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await.unwrap();
                    *client_mutex.lock().unwrap() = Some(client);
                    if let Err(reason) = connection.await {
                        println!("Error in DatabaseMiddleware: {reason}");
                    }
                }
            })
        };
    
        Self {
            client_mutex,
            handle,
        }
    }

    pub fn service_fn<T>(&self, f: T) -> DatabaseMiddlewareServiceFn<T> {
        DatabaseMiddlewareServiceFn {
            f,
            client_mutex: self.client_mutex.clone(),
        }
    }

    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.handle.await
    }
}

pub struct DatabaseMiddlewareServiceFn<T> {
    f: T,
    client_mutex: Arc<Mutex<Option<tokio_postgres::Client>>>,
}
impl<T, F, Request, R, E> tower_service::Service<Request> for DatabaseMiddlewareServiceFn<T>
where
    T: FnMut((Request, Arc<Mutex<Option<tokio_postgres::Client>>>)) -> F,
    F: Future<Output = Result<R, E>>,
{
    type Response = R;
    type Error = E;
    type Future = F;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), E>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        (self.f)((req, self.client_mutex.clone()))
    }
}