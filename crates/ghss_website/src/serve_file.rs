use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tide::{Body, Endpoint, Request, Response, Result, Route};

struct ServeFile {
    path: PathBuf,
}

impl<State> Endpoint<State> for ServeFile
where
    State: Clone + Send + Sync + 'static,
{
    fn call<'life0, 'async_trait>(
        &'life0 self,
        _req: Request<State>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let res: Response = Body::from_file(&self.path).await?.into();
            Ok(res)
        })
    }
}

pub trait RouteExt {
    fn serve_file(&mut self, path: impl AsRef<Path>);
}

impl<'a, State> RouteExt for Route<'a, State>
where
    State: Clone + Send + Sync + 'static,
{
    fn serve_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_owned();
        self.get(ServeFile { path });
    }
}
